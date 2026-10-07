mod answers;
mod banned;
mod blame;
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

use crate::depends;
use crate::ledger::Ledger;
use crate::rules;
use crate::msg::{self, Actor};
use crate::out::{self, Line};
use crate::record::{self, RecordFile};
use crate::repo::{self, Source};
use crate::state;
use crate::terms as terms_mod;
use crate::view;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// One finding of a category of `check` that is not a rule `approve` or
/// `ask` judge (the ledger, the install, the ban list, the tests): its
/// category and its message, the move that clears it inside.
struct Failure {
    check: &'static str,
    message: String,
}

impl Failure {
    pub(super) fn finding(self) -> rules::Finding {
        rules::Finding::new(self.check, self.message)
    }
}

fn push_failures(out: &mut Vec<rules::Finding>, fs: Vec<Failure>) {
    out.extend(fs.into_iter().map(Failure::finding));
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
    let unreadable_failures: Vec<rules::Finding> = unreadable
        .into_iter()
        .map(|(f, e)| rules::Finding::new("parse", msg::topic_file_unreadable(&state::rel_path(root, &f), e)))
        .collect();
    let mut notes: Vec<rules::Finding> = Vec::new();

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
    // `depends:` entries in memory: captured here, ahead of that mutation,
    // keyed by `rel` and threaded down to every check that compares against
    // the ledger (`file_approval`, `ids::check_ids`'s stale-plan filter)
    // instead of letting them recompute the hash from the (by-then-
    // normalized) `RecordFile`.
    let raw_hashes: HashMap<String, (Vec<String>, String)> = records
        .iter()
        .map(|rf| (rf.rel.clone(), (record::question_hashes(rf), record::context_hash(rf))))
        .collect();
    // The files awaiting approval: a finding on one of them is a note (per
    // "What does check do with a fault on a file awaiting approval?").
    // Read off each file as written, before the normalize pass below: the
    // same scope `approve` reads for the planning rules.
    let awaiting: Vec<String> = records
        .iter()
        .filter(|rf| rules::planning_rules_apply(&led, rf, &rf.rel))
        .map(|rf| rf.rel.clone())
        .collect();

    // Heading-shaped `depends:` entries that name no heading in their own
    // file but resolve to exactly one current decision elsewhere are
    // rewritten to `Id` entries in memory here, before any finding is
    // judged below -- the same normalization `approve` runs on the file it
    // is approving, so `check` never reports a legitimate cross-file
    // reference as the file-wide `derive_error` a plain, unresolved `Text`
    // heading dep always earns.
    let as_loaded = records.clone();
    for rf in &mut records {
        depends::normalize_cross_file_deps(rf, &terms_list, &phrase_lookup, &cross_file_headings, &led);
    }

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
    // Every finding each topic file earns on its own: the one judgement
    // `approve` refuses on (`rules::judge_topic_file`, per "Does a command
    // refuse what check passes?", fe14991d A), one `Vec` per record in
    // `records`' order. `parse`/`tells`/`plan` below file them by rule.
    // What the planning of every file reads of the whole repository (the
    // reader's rows, the heading indexes), read once for the run.
    // Built only when a file awaits approval: the planning rules judge no
    // other file (`rules::planning_rules_apply`).
    let shared = (!awaiting.is_empty()).then(|| crate::plan::Shared::new(root, &records, &terms_list, &led));
    let per_record_findings: Vec<Vec<rules::Finding>> = records
        .iter()
        .zip(&replaced_per_record)
        .map(|(rf, replaced)| {
            let judge = rules::TopicFileJudge {
                root,
                rel: &rf.rel,
                rf,
                all: &records,
                terms_list: &terms_list,
                phrase_lookup: &phrase_lookup,
                cross_file_headings: &cross_file_headings,
                names: &names,
                replaced,
                led: &led,
                planning: awaiting.contains(&rf.rel),
                shared: shared.as_ref(),
            };
            topic_file_findings(&judge)
        })
        .collect();
    let present_ids: std::collections::HashSet<String> = records
        .iter()
        .flat_map(|rf| rf.sections.iter().filter_map(|s| s.id.clone()))
        .collect();
    let removed_files: Vec<(String, Vec<crate::moves::RemovalHold>)> =
        rules::removal_awaiting(&led, &|rel| source.exists(root, rel))
            .into_iter()
            .map(|rel| {
                let holds = rules::removal_holds(root, &rel, &led, &present_ids).holds;
                (rel, holds)
            })
            .collect();
    // A removed file differs from its approval: what keeps `approve` from
    // taking its removal is a note, with the move that clears it.
    for (rel, _) in &removed_files {
        notes.extend(rules::judge_removed_topic_file(root, rel, &led, &present_ids));
    }
    // An answered id of a removed file is named by the ids category with the
    // file's removal move, so the missing-answered finding leaves it out.
    let in_removed_file: std::collections::HashSet<String> = led
        .rows
        .iter()
        .filter(|r| r.id != "-" && r.event == "answered")
        .filter(|r| {
            led.latest_for_id("approved", &r.id)
                .is_some_and(|a| removed_files.iter().any(|(rel, _)| *rel == a.detail))
        })
        .map(|r| r.id.clone())
        .collect();
    let missing_answered = rules::missing_answered_refusals_except(&records, &led, &in_removed_file);
    let mut failures: Vec<rules::Finding> = unreadable_failures;
    let (parse_failures, parse_notes) = parse::check_parse(&records, &per_record_findings, &awaiting, &led);
    push_failures(&mut failures, parse_failures);
    push_failures(&mut notes, parse_notes);
    push_failures(&mut failures, ids::check_ids(&records, &led, &missing_answered, &raw_hashes, &removed_files));
    push_failures(&mut failures, ledger::check_ledger(&led, &records, root, &ledger_content, since));
    push_failures(&mut failures, reader::check_reader(root, &source, since));
    push_failures(&mut failures, banned::check_banned_ledger(&led, root, &source));
    push_failures(&mut failures, answers::check_answers(root, &records, &as_loaded, &terms_list, &led));
    push_failures(&mut failures, banned::check_banned(root, staged, message, &source));
    push_failures(&mut failures, install::check_install(root, &source));
    let (plan_failures, plan_notes) =
        plan::check_plan(root, &source, &records, &led, &per_record_findings, &raw_hashes, &awaiting);
    push_failures(&mut failures, plan_failures);
    push_failures(&mut notes, plan_notes);
    push_failures(&mut failures, terms::check_terms(root, &records, &led, &source, &approved_rels));
    let (blame_failures, blame_notes) = blame::check_blame(root, &source, &records, &terms_list, &led);
    push_failures(&mut failures, blame_failures);
    notes.extend(blame_notes);
    let (test_failures, untested) = tests::check_tests(root, &records, &led, tests, &source);
    push_failures(&mut failures, test_failures);
    let (tells_failures, tells_notes) = tells::check_tells(&records, &per_record_findings, &awaiting, &led);
    push_failures(&mut failures, tells_failures);
    push_failures(&mut notes, tells_notes);
    // Per "What does check do with an untested current decision?": reported
    // on every run, never a failure, each with the move that builds it.
    notes.extend(untested);

    view::CheckReport { failures, notes }
}

/// What `check` reports of one topic file of the records: the judgement
/// `approve` refuses on (`rules::judge_topic_file`), less the naming and
/// heading rules `plan::check_plan` reports for every file of the
/// directory itself (a name, a heading twice in one file, a heading in two
/// files, each a note or a failure by which question copies).
fn topic_file_findings(j: &rules::TopicFileJudge) -> Vec<rules::Finding> {
    let mut own: std::collections::HashSet<String> =
        rules::own_name_and_heading_refusals(j.rel, j.rf.topic.as_deref(), &j.rf.sections).into_iter().collect();
    own.extend(rules::cross_file_heading_refusals(j.rel, j.rf, j.all, j.led));
    rules::judge_topic_file(j).into_iter().filter(|f| !own.contains(&f.message)).collect()
}

/// The arguments `check` ran with, as the command line names them: what a
/// refusal's move names to run again (per "Does a refusal name the command
/// that clears it?").
fn check_args(staged: bool, message: Option<&Path>, since: Option<&str>) -> String {
    let mut args: Vec<String> = Vec::new();
    if staged {
        args.push("--staged".to_string());
    }
    if let Some(m) = message {
        args.push(format!("--message {}", m.display()));
    }
    if let Some(s) = since {
        args.push(format!("--since {}", s));
    }
    args.join(" ")
}

/// `push` (`--push`) short-circuits to `push::run`, which reads git's
/// pre-push stdin and judges only that one ledger comparison, per ref line
/// -- never the rest of what `check` reports (per "Does check look at the
/// ledger before a push?", 19e1c023-a678-4842-a414-bb0e45c41601 P). Every
/// other combination runs the plain report, `since` (`--since <commit>`)
/// adding its one extra ledger comparison on top.
///
/// Every line is a finding with its move, the failures first, then the
/// notes; a pass prints nothing, by hand or in a hook (per "Does check
/// print a line that names no move?", 3fc35d4f A).
pub fn run(staged: bool, message: Option<PathBuf>, since: Option<String>, push: bool) -> i32 {
    if push {
        return push::run();
    }

    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            let args = check_args(staged, message.as_deref(), since.as_deref());
            out::eprint(&[stopped(msg::not_a_repo("check", &args, Actor::Agent))]);
            return 1;
        }
    };

    // The commit-msg hook: the staged files were judged by the pre-commit
    // hook of the same commit, so only the message is read, for banned
    // words (per "What does the ban scan read?": the staged files and the
    // commit message before every commit). Each finding prints once a commit.
    if let Some(m) = message.as_deref() {
        let findings: Vec<Line> = banned::check_banned(&root, false, Some(m), &Source::for_check(false, true))
            .into_iter()
            .map(|f| Line::Finding(f.finding()))
            .collect();
        out::print(&findings);
        return if findings.is_empty() { 0 } else { 1 };
    }
    let report = build(&root, staged, None, since.as_deref(), None);
    let exit_code = if report.failures.is_empty() { 0 } else { 1 };
    let lines: Vec<Line> = report.failures.into_iter().chain(report.notes).map(Line::Finding).collect();
    out::print(&lines);
    exit_code
}

/// A finding on what `check` was given rather than on the records (no
/// repository, unreadable input), filed under `check`; `message` may carry
/// that name in front already.
pub(super) fn stopped(message: String) -> Line {
    let message = message.strip_prefix("check: ").map(str::to_string).unwrap_or(message);
    Line::Finding(rules::Finding::new("check", message))
}

/// Whether a finding `r` on `rf` is a note rather than a failure (per
/// "What does check do with a fault on a file awaiting approval?", 05141f49
/// A): the file awaits approval (`draft`), the finding is of a kind a
/// draft can still fix (`tells`, `plan`, or any `parse` finding: the file's
/// shape, a stray line, a line the tool writes), and it is not about a
/// question with force. Anything else fails check; so does every finding
/// on an approved file.
fn is_note(rf: &RecordFile, r: &rules::Finding, draft: bool, led: &Ledger) -> bool {
    if !draft {
        return false;
    }
    if !matches!(r.rule, "tells" | "plan" | "parse") {
        return false;
    }
    // Every question the finding is about must still be able to change; a
    // finding about none is about the file as a whole.
    !r.sections.iter().filter_map(|i| rf.sections.get(*i)).any(|s| rules::has_force(rf, s, led))
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
