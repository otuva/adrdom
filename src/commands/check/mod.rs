mod answers;
mod banned;
mod done;
mod ids;
mod install;
mod ledger;
mod moot;
mod parse;
mod plan;
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

/// Loads one topic file through `source` instead of `record::load_in`,
/// which always reads the filesystem: the one place `check` builds a
/// `RecordFile` from bytes, so every category below judges the same
/// staged-or-working-tree text (per "Does check at a commit read the
/// staged files?", 34e81f57 A). `path` is one of `source`'s own file
/// listing (`repo::list_decision_files_via`), so it may not exist on disk
/// at all (a file only staged, never `git add`ed to disk... never mind,
/// always on disk if staged; but never committed either) -- only
/// `source.read` is ever asked for its bytes.
fn load_record(root: &Path, path: &Path, source: &Source) -> Result<RecordFile, String> {
    let rel = state::rel_path(root, path);
    match source.read(root, &rel) {
        Some(content) => Ok(record::build(path.to_path_buf(), rel, record::split_lines(&content))),
        None => Err("not found".to_string()),
    }
}

struct Failure {
    check: String,
    /// The decision a failure is about, kept so the page can group by it.
    #[allow(dead_code)]
    id: Option<String>,
    message: String,
}

fn push_failures(out: &mut Vec<String>, fs: Vec<Failure>) {
    for f in fs {
        out.push(format!("{}: {}", f.check, f.message));
    }
}

/// Everything `check` reports, computed against `root` with the default
/// (non-staged, no message file) banned-word scan. Used by `run` (which
/// layers the CLI's `--staged`/`--message` modes on top) and by the `serve`
/// page, which always wants the plain repo-wide scan.
/// The full check report for the owner's page, reusing a test index the
/// caller already computed so the repo's tracked files are scanned once.
pub fn collect_with_tests(root: &Path, tests: &HashMap<String, Vec<String>>) -> view::CheckReport {
    build(root, false, None, Some(tests))
}

fn build(
    root: &Path,
    staged: bool,
    message: Option<&Path>,
    tests: Option<&HashMap<String, Vec<String>>>,
) -> view::CheckReport {
    // Picked once per run and threaded through every category below (per
    // "Does check at a commit read the staged files?", 34e81f57 A): the
    // commit-msg hook's `--message` run judges the same staged tree the
    // pre-commit hook already saw -- it is "at a commit" too, even though
    // its own `staged` flag is false -- so it counts here alongside
    // `--staged` itself.
    let source = Source::for_check(staged, message.is_some());

    let ledger_rel = "adrdom/state/ledger.tsv";
    let ledger_content = source.read(root, ledger_rel).unwrap_or_default();
    let led = crate::ledger::parse(&ledger_content);

    let files = repo::list_decision_files_via(root, &source);
    let mut records = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    for f in &files {
        match load_record(root, f, &source) {
            Ok(rf) => records.push(rf),
            Err(e) => notes.push(msg::topic_file_unreadable(&f.display().to_string(), e)),
        }
    }

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
    let raw_hashes: HashMap<String, (String, String)> = records
        .iter()
        .map(|rf| (rf.rel.clone(), (record::plan_hash(rf), record::context_hash(rf))))
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
    let per_record_refusals: Vec<Vec<actions::Refusal>> = records
        .iter()
        .map(|rf| actions::record_refusals(&rf.rel, rf, &terms_list, &phrase_lookup, &cross_file_headings, &led))
        .collect();
    let missing_answered = actions::missing_answered_refusals(&records, &led);

    let mut failures: Vec<String> = Vec::new();
    push_failures(&mut failures, parse::check_parse(&per_record_refusals));
    push_failures(&mut failures, ids::check_ids(&records, &led, &missing_answered, &raw_hashes));
    push_failures(&mut failures, ledger::check_ledger(&led, &records, root, &ledger_content));
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
        &terms_list,
        &phrase_lookup,
        &cross_file_headings,
        &raw_hashes,
    );
    push_failures(&mut failures, plan_failures);
    let (terms_failures, terms_awaiting, terms_notes) = terms::check_terms(root, &records, &led, &source);
    push_failures(&mut failures, terms_failures);
    let (test_failures, tested_count) = tests::check_tests(root, &records, &led, tests, &source);
    push_failures(&mut failures, test_failures);
    push_failures(&mut failures, tells::check_tells(root, &records, &led, &per_record_refusals));

    notes.extend(moot::check_moot(&records, &led));

    for rf in &records {
        if let Some(tiers) = &rf.tiers {
            // Live counts only (per "Does the tool cap the questions in a
            // tier?"): a dead (refused/superseded/
            // withdrawn) question no longer holds its tier for the cap,
            // and this note is read against that same cap.
            let live = state::live_tiers(&led, rf, tiers);
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
    }
    if terms_awaiting {
        notes.push(msg::terms_awaiting());
    }
    notes.extend(terms_notes);
    for d in done::check_done(&records, &led, &raw_hashes) {
        notes.push(msg::done(&d));
    }
    notes.push(msg::tested_count(tested_count));

    view::CheckReport { failures, notes }
}

pub fn run(staged: bool, message: Option<PathBuf>) -> i32 {
    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            eprintln!("{}", msg::not_a_repo("check", Actor::Agent));
            return 1;
        }
    };

    let report = build(&root, staged, message.as_deref(), None);
    let exit_code = if report.failures.is_empty() { 0 } else { 1 };

    for f in &report.failures {
        println!("{}", f);
    }
    for n in &report.notes {
        println!("{}", n);
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
    raw_hashes: &HashMap<String, (String, String)>,
    led: &Ledger,
) -> (String, bool, bool) {
    let rel = rf.rel.clone();
    let (plan_h, ctx_h) = raw_hashes.get(&rel).unwrap_or_else(|| {
        panic!(
            "file_approval: {} missing from raw_hashes, though raw_hashes is built from the same records list this is called on",
            rel
        )
    });
    let plan_ok = state::plan_approved_hash(led, &rel, plan_h);
    let ctx_ok = state::context_approved_hash(led, &rel, ctx_h);
    (rel, plan_ok, ctx_ok)
}
