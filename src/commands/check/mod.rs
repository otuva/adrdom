mod answers;
mod banned;
mod blame;
mod done;
mod ids;
mod install;
mod ledger;
mod parse;
mod plan;
mod push;
mod reader;
mod tells;
mod terms;
mod tests;

pub(crate) use banned::collect_targets;
pub use tests::test_index;

use crate::actions;
use crate::depends;
use crate::ledger::Ledger;
use crate::msg::{self, Actor};
use crate::record::{self, RecordFile};
use crate::repo::{self, Source};
use crate::state;
use crate::terms as terms_mod;
use crate::view;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

struct Failure {
    check: String,
    message: String,
}

fn push_failures(out: &mut Vec<String>, fs: Vec<Failure>) {
    for f in fs {
        out.push(format!("{}: {}", f.check, f.message));
    }
}

/// The full check report for the owner's page, reusing a test index the
/// caller already computed so the repo's tracked files are scanned once.
/// Always runs the plain repo-wide scan (non-staged, no message file):
/// `serve` has no commit to check "at", unlike `run`.
pub fn collect_with_tests(root: &Path, tests: &HashMap<String, Vec<String>>) -> view::CheckReport {
    build(root, false, None, None, Some(tests))
}

/// Everything `check` reports, computed against `root`. `run` passes
/// `staged`/`message` on for the pre-commit/commit-msg hook modes, and
/// `since` for `--since <commit>` (per "Does check in CI compare the
/// ledger with the commit before the push?", a26876ba-5789-4d05-965e-853d00d9edd2
/// C); `collect_with_tests` always passes the plain non-staged, no-message,
/// no-`since` scan.
fn build(
    root: &Path,
    staged: bool,
    message: Option<&Path>,
    since: Option<&str>,
    tests: Option<&HashMap<String, Vec<String>>>,
) -> view::CheckReport {
    // Picked once per run and threaded through every category below (per
    // "Does check at a commit read the staged files?", 34e81f57 A): the
    // commit-msg hook's `--message` run judges the same staged tree the
    // pre-commit hook already saw -- it is "at a commit" too, even though
    // its own `staged` flag is false -- so it counts here alongside
    // `--staged` itself.
    let source = Source::for_check(staged, message.is_some());

    let ledger_rel = repo::LEDGER_PATH;
    let ledger_content = source.read(root, ledger_rel).unwrap_or_default();
    let led = crate::ledger::parse(&ledger_content);

    // A topic file that cannot be read is a failure: nothing in it is
    // judged, and a pass would let it through with its decisions unread.
    let (mut records, unreadable) = record::load_all_via(root, &source);
    let unreadable_failures: Vec<String> = unreadable
        .into_iter()
        .map(|(f, e)| msg::topic_file_unreadable(&state::rel_path(root, &f), e))
        .collect();
    let mut notes: Vec<String> = Vec::new();

    // Terms minted in memory: a terms file that has never been
    // approved carries no ids on disk yet, and `phrase_lookup` only
    // resolves a term that has one. Computed once here, shared by every
    // category below that resolves a `depends:` entry, rather than each
    // reloading/minting its own copy.
    let terms_list: Vec<terms_mod::Term> = state::load_terms_via(root, &source)
        .map(|tf| terms_mod::minted_copy(&tf).terms)
        .unwrap_or_default();
    let phrase_lookup = terms_mod::phrase_lookup(&terms_list);
    // Every current decision's heading text, repo-wide (the cross-file gap
    // "Does a reference in a topic file carry the identifier of what it
    // names?" left open): built once here too, shared the same way
    // `terms_list`/`phrase_lookup` already are. Built from `records`,
    // already loaded above, rather than re-reading every topic file a
    // second time off disk.
    let cross_file_headings = depends::cross_file_heading_index(&records, &led);

    // Plan/context approval state must be read off each file exactly as it
    // sits on disk, before the normalize pass below mutates its parsed
    // `depends:` entries in memory (per "..."): captured here, ahead of
    // that mutation, keyed by `rel` and threaded down to every check that
    // compares against the ledger (`file_approval`, `ids::check_ids`'s
    // stale-plan filter) instead of letting them recompute the hash from
    // the (by-then-normalized) `RecordFile`.
    let raw_hashes: HashMap<String, (Vec<String>, String)> = records
        .iter()
        .map(|rf| (rf.rel.clone(), (record::question_hashes(rf), record::context_hash(rf))))
        .collect();

    // Heading-shaped `depends:` entries that name no heading in their own
    // file but resolve to exactly one current decision elsewhere are
    // rewritten to `Id` entries in memory here, before any refusal is
    // computed below -- the same normalization `approve` runs on the file
    // it is approving, so `check` never reports a legitimate cross-file
    // reference as the file-wide `derive_error` a plain, unresolved `Text`
    // heading dep always earns. An entry matching more than one current
    // decision elsewhere becomes that file's `derive_error`
    // (`DeriveError::AmbiguousHeading`) instead, which `record_refusals`
    // below already turns into a "plan" category failure -- no side list
    // to fold in here any more.
    for rf in &mut records {
        depends::normalize_cross_file_deps(rf, &terms_list, &phrase_lookup, &cross_file_headings, &led);
    }

    // Every refusal each topic file earns on its own (shared with
    // `approve`'s `do_approve_record`), one `Vec` per record in `records`'
    // order, plus the cross-file "answered id missing from every topic
    // file" refusal, computed once for the whole repo. `parse`/`tells`/
    // `plan` pull their failures from these instead of re-deriving the
    // same shape/tells/depends checks `approve` already refuses on; `ids`
    // pulls the missing-answered-id lines the same way.
    let names = crate::view::heading_texts(&records);
    // Which plans are approved comes from `raw_hashes`, taken before the
    // normalize pass; computed once and shared with `check_terms`. The
    // replaced-id set is built once and extended per file with that file's
    // own `supersedes:` lines.
    let approved_rels: std::collections::HashSet<String> = raw_hashes
        .iter()
        .filter(|(rel, (plan_h, _))| state::plan_approved_hashes(&led, rel, plan_h))
        .map(|(rel, _)| rel.clone())
        .collect();
    let replaced_base = depends::replaced_base(&records, &approved_rels, &cross_file_headings, &led);
    let replaced_per_record: Vec<std::collections::HashSet<String>> = records
        .iter()
        .map(|rf| depends::replaced_ids(&replaced_base, rf, &cross_file_headings, &led))
        .collect();
    let per_record_refusals: Vec<Vec<actions::Refusal>> = records
        .iter()
        .zip(&replaced_per_record)
        .map(|(rf, replaced)| {
            actions::record_refusals(&rf.rel, rf, &terms_list, &phrase_lookup, &cross_file_headings, &names, &records, replaced, &led)
        })
        .collect();
    let missing_answered = actions::missing_answered_refusals(&records, &led);

    let mut failures: Vec<String> = unreadable_failures;
    push_failures(&mut failures, parse::check_parse(&per_record_refusals));
    push_failures(&mut failures, ids::check_ids(&records, &led, &missing_answered, &raw_hashes));
    push_failures(&mut failures, ledger::check_ledger(&led, &records, root, &ledger_content, since));
    push_failures(&mut failures, reader::check_reader(root, &source, since));
    push_failures(&mut failures, banned::check_banned_ledger(&led, root, &source));
    push_failures(&mut failures, answers::check_answers(&records, &led));
    push_failures(&mut failures, banned::check_banned(root, staged, message, &source));
    push_failures(&mut failures, install::check_install(root, &source));
    let (plan_failures, awaiting) = plan::check_plan(
        root,
        &source,
        &records,
        &led,
        &per_record_refusals,
        &raw_hashes,
    );
    push_failures(&mut failures, plan_failures);
    let (terms_failures, terms_notes) = terms::check_terms(root, &records, &led, &source, &approved_rels);
    push_failures(&mut failures, terms_failures);
    let (blame_failures, blame_notes) = blame::check_blame(root, &source, &records, &led);
    push_failures(&mut failures, blame_failures);
    let (test_failures, tested_count, test_notes) =
        tests::check_tests(root, &records, &led, tests, &source);
    push_failures(&mut failures, test_failures);
    push_failures(&mut failures, tells::check_tells(&per_record_refusals));


    for (rf, replaced) in records.iter().zip(&replaced_per_record) {
        if let Some(tiers) = &rf.tiers {
            // Live counts only (per "Does the tool cap the questions in a
            // tier?"): a dead (refused/superseded/
            // withdrawn) question no longer holds its tier for the cap,
            // and this note is read against that same cap.
            let live = state::live_tiers(&led, rf, tiers, replaced);
            if live.is_empty() {
                continue;
            }
            let sizes = record::tier_sizes(&live)
                .into_iter()
                .map(|(t, n)| format!("{} ({})", t, n))
                .collect::<Vec<_>>()
                .join(" ");
            notes.push(msg::tiers_note_check(&rf.rel, &sizes));
        }
    }

    if !awaiting.is_empty() {
        notes.push(msg::plan_awaiting(awaiting.len(), &awaiting.join(", ")));
        for rf in records.iter().filter(|rf| awaiting.contains(&rf.rel)) {
            if let Some((hashes, _)) = raw_hashes.get(&rf.rel) {
                let (changed, struck) = plan::changed_questions(rf, hashes, &led);
                if !changed.is_empty() || !struck.is_empty() {
                    notes.push(msg::plan_changed(&rf.rel, &changed, &struck));
                }
            }
        }
    }
    notes.extend(test_notes);
    notes.extend(terms_notes);
    notes.extend(blame_notes);
    for d in done::check_done(&records, &led, &raw_hashes) {
        notes.push(msg::done(&d));
    }
    notes.push(msg::tested_count(tested_count));

    view::CheckReport { failures, notes }
}

/// `push` (`--push`) short-circuits to `push::run`, which reads git's
/// pre-push stdin and judges only that one ledger comparison, per ref line
/// -- never the rest of what `check` reports (per "Does check look at the
/// ledger before a push?", 19e1c023-a678-4842-a414-bb0e45c41601 P). Every
/// other combination runs the plain report, `since` (`--since <commit>`)
/// adding its one extra ledger comparison on top.
///
/// `quiet` (`--quiet`) prints the failures and leaves out the notes: what
/// the commit hooks run, so a commit that passes prints nothing and one
/// that fails prints only what failed.
pub fn run(staged: bool, message: Option<PathBuf>, since: Option<String>, push: bool, quiet: bool) -> i32 {
    if push {
        return push::run();
    }

    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            eprintln!("{}", msg::not_a_repo("check", "", Actor::Agent));
            return 1;
        }
    };

    let report = build(&root, staged, message.as_deref(), since.as_deref(), None);
    let exit_code = if report.failures.is_empty() { 0 } else { 1 };

    for f in &report.failures {
        println!("{}", f);
    }
    if !quiet {
        for n in &report.notes {
            println!("{}", n);
        }
    }

    exit_code
}

/// An id whose latest state is `superseded` or `withdrawn` is no longer
/// demanded in any topic file (4.13/6.1): the topic file it once lived in was
/// allowed to drop it.
fn dropped_state(led: &Ledger, id: &str) -> bool {
    matches!(
        led.latest_state(id).map(|r| r.event.as_str()),
        Some("superseded") | Some("withdrawn")
    )
}

/// A file's plan/context approval state: its ledger-relative path, whether
/// its current plan hash matches the latest `approved` row for that path,
/// and likewise for context/`context`. `raw_hashes` is `rel -> (plan_hash,
/// context_hash)`, computed by `build` before its normalize pass mutates
/// `rf`'s parsed `depends:` entries in memory -- approval state must
/// always be read off the file exactly as it sits on disk (per "..."), so
/// this reads that precomputed hash rather than recomputing one from `rf`,
/// which by the time any caller here runs has already been normalized.
/// Keyed and looked up by `rf.rel` on both sides (the same field
/// `raw_hashes` itself was built from, in `build`, off the very same
/// `records` list every caller here still shares) -- never `rel_path(root,
/// &rf.path)` recomputed afresh, which risks the two disagreeing. A miss
/// is a bug in that shared construction, not a case to paper over with a
/// silent recompute from the (by-then-normalized) `rf`, so it panics
/// instead of guessing.
fn file_approval(
    rf: &RecordFile,
    raw_hashes: &HashMap<String, (Vec<String>, String)>,
    led: &Ledger,
) -> (String, bool, bool) {
    let rel = rf.rel.clone();
    let (plan_h, ctx_h) = raw_hashes.get(&rel).unwrap_or_else(|| {
        panic!(
            "file_approval: {} missing from raw_hashes, though raw_hashes is built from the same records list this is called on",
            rel
        )
    });
    let plan_ok = state::plan_approved_hashes(led, &rel, plan_h);
    let ctx_ok = state::context_approved_hash(led, &rel, ctx_h);
    (rel, plan_ok, ctx_ok)
}
