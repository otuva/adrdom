mod answers;
mod banned;
mod blame;
mod ids;
mod install;
mod ledger;
mod parse;
mod plan;
mod planner;
mod push;
mod reader;
mod terms;
mod tests;

pub(crate) use banned::collect_targets;
pub use tests::test_index;

use crate::rules::{Class, Command};
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

/// One finding of a category of `check`: a `rules::Finding` filed under its
/// category, the move that clears it inside its message. One type for every
/// finding check prints, the rules' and its own categories' alike.
type Failure = rules::Finding;

fn push_failures(out: &mut Vec<rules::Finding>, fs: Vec<Failure>) {
    out.extend(fs);
}

/// The full check report for the owner's page, reusing a test index the
/// caller already computed so the repo's tracked files are scanned once.
/// Always runs the plain repo-wide scan (non-staged, no message file):
/// `serve` has no commit to check "at", unlike `run`.
pub fn collect_with_tests(root: &Path, tests: &HashMap<String, Vec<String>>) -> view::CheckReport {
    build(root, false, None, None, Some(tests), None)
}

/// The full check report for the owner's page from the records the request
/// already loaded: the ledger, the topic files, the terms and what the
/// planning shares are taken from `held`, so a request reads and indexes
/// them once and the report is the one `check` prints.
///
/// `judged` holds each topic file's own judgement
/// (`moves::of_topic_file_judged`) in the order of `held.topic_files`: the
/// page computed it for the cards, and `check` files the same findings by
/// rule, less the directory ones it reads off the directory itself, so a
/// request judges each file once.
pub(crate) fn collect_from(held: &crate::commands::show::Records, judged: &[&[rules::Finding]]) -> view::CheckReport {
    build(&held.root, false, None, None, Some(&held.tests), Some((held, judged)))
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
    held: Option<(&crate::commands::show::Records, &[&[rules::Finding]])>,
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
    let parsed_led;
    let (led, records, unreadable_stops): (&Ledger, Vec<RecordFile>, Vec<String>) = match held {
        Some((h, _)) => (
            &h.led,
            h.topic_files.clone(),
            h.unreadable_topic_files
                .iter()
                .map(|(rel, e)| msg::topic_file_unreadable(rel, e))
                .collect::<Vec<_>>(),
        ),
        None => {
            parsed_led = crate::ledger::parse(&ledger_content);
            // A topic file that cannot be read is a failure: nothing in it is
            // judged, and a pass would let it through with its decisions unread.
            let (records, unreadable) = record::load_all_via(root, &source);
            let unreadable_stops: Vec<String> = unreadable
                .into_iter()
                .map(|(f, e)| msg::topic_file_unreadable(&state::rel_path(root, &f), e))
                .collect();
            (&parsed_led, records, unreadable_stops)
        }
    };
    let mut notes: Vec<rules::Finding> = Vec::new();

    // Terms minted in memory: a terms file that has never been
    // approved carries no ids on disk yet, and `phrase_lookup` only
    // resolves a term that has one. Computed once here, shared by every
    // category below that resolves a `depends:` entry, rather than each
    // reloading/minting its own copy.
    let terms_list: Vec<terms_mod::Term> = match held {
        Some((h, _)) => h.terms_list.clone(),
        None => state::load_terms_via(root, &source)
            .map(|tf| terms_mod::minted_copy(&tf).terms)
            .unwrap_or_default(),
    };
    let phrase_lookup = terms_mod::phrase_lookup(&terms_list);
    // Every current decision's heading text, repo-wide (the cross-file gap
    // "What does a reference in a topic file carry?" left open): built once here too, shared the same way
    // `terms_list`/`phrase_lookup` already are. Built from `records`,
    // already loaded above, rather than re-reading every topic file a
    // second time off disk.
    let cross_file_headings = depends::cross_file_heading_index(&records, &led);

    // Plan/context approval state must be read off each file exactly as it
    // sits on disk: captured here, keyed by `rel` and threaded down to
    // every check that compares against the ledger (`file_approval`,
    // `ids::check_ids`'s stale-plan filter).
    let raw_hashes: HashMap<String, (Vec<String>, String)> = records
        .iter()
        .map(|rf| (rf.rel.clone(), (record::question_hashes(rf), record::context_hash(rf))))
        .collect();
    // The files awaiting approval: a finding on one of them is a note (per
    // "What does check do with a fault on a file awaiting approval?").
    // Read off each file as written: the
    // same scope `approve` reads for the planning rules.
    let awaiting: Vec<String> = records
        .iter()
        .filter(|rf| rules::planning_rules_apply(&led, rf, &rf.rel))
        .map(|rf| rf.rel.clone())
        .collect();

    let names = crate::view::heading_texts(&records);
    // Which plans are approved comes from `raw_hashes`, computed once and shared with `check_terms`. The
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
    let built = (held.is_none() && !awaiting.is_empty())
        .then(|| crate::plan::Shared::new_via(root, &records, &terms_list, &led, &source));
    let shared = match held {
        Some((h, _)) if !awaiting.is_empty() => Some(&h.index().planning),
        _ => built.as_ref(),
    };
    // Every blame file, read once through the run's source: the `blame:`
    // lines are judged against it, and `check_blame` judges the files.
    let read_blame;
    let (blame_index, blame_unreadable): (&crate::blame::Index, Vec<String>) = match held {
        Some((h, _)) => (&h.index().blame, h.unreadable_blame_files.iter().map(|(rel, _)| rel.clone()).collect()),
        None => {
            read_blame = crate::blame::Index::read(root, &source, &led);
            (&read_blame.0, read_blame.1)
        }
    };
    let per_record_findings: Vec<Vec<rules::Finding>> = match held {
        Some((_, judged)) if judged.len() == records.len() => judged
            .iter()
            .map(|fs| fs.iter().filter(|f| !matches!(f.subject, rules::Subject::Directory(_))).cloned().collect())
            .collect(),
        _ => records
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
                blame: blame_index,
                led: &led,
                planning: awaiting.contains(&rf.rel),
                shared,
                directory: false,
            };
            topic_file_findings(&judge)
        })
        .collect(),
    };
    let present_ids: std::collections::HashSet<String> =
        rules::held_ids(&records).into_iter().map(str::to_string).collect();
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
    // The rules on the topic files as a whole (`rules::judge_records`).
    let missing_answered = rules::judge_records_except(&records, &led, &in_removed_file);
    let mut failures: Vec<rules::Finding> = Vec::new();
    let mut stops: Vec<String> = unreadable_stops;
    let (parse_failures, parse_notes) = parse::check_rule(Class::Parse, &records, &per_record_findings, &awaiting, &led);
    push_failures(&mut failures, parse_failures);
    push_failures(&mut notes, parse_notes);
    push_failures(&mut failures, ids::check_ids(&records, &led, &missing_answered, &raw_hashes, &removed_files));
    push_failures(&mut failures, ledger::check_ledger(&led, &records, root, &ledger_content, since, &mut stops));
    push_failures(&mut failures, reader::check_reader(root, &source, since));
    push_failures(&mut failures, banned::check_banned_ledger(&led, root, &source));
    push_failures(&mut failures, answers::check_answers(root, &records, &terms_list, &led, &source));
    push_failures(&mut failures, banned::check_banned(root, staged, message, &source, &mut stops));
    push_failures(&mut failures, install::check_install(root, &source));
    let (plan_failures, plan_notes) =
        plan::check_plan(root, &source, &records, &led, &per_record_findings, &awaiting);
    push_failures(&mut failures, plan_failures);
    push_failures(&mut notes, plan_notes);
    push_failures(&mut failures, terms::check_terms(root, &records, &led, &source, &approved_rels));
    let (blame_failures, blame_notes) = blame::check_blame(root, &source, blame_index, &blame_unreadable, &mut stops);
    push_failures(&mut failures, blame_failures);
    notes.extend(blame_notes);
    if staged || message.is_some() {
        push_failures(&mut failures, planner::check_commit(root));
    }
    let untested = tests::check_tests(root, &records, &led, tests, &source, crate::session::role(root));
    let (tells_failures, tells_notes) = parse::check_rule(Class::Tells, &records, &per_record_findings, &awaiting, &led);
    push_failures(&mut failures, tells_failures);
    push_failures(&mut notes, tells_notes);
    // Per "What does check do with an untested current decision?": reported
    // on every run, never a failure, each with the move that builds it.
    notes.extend(untested);

    view::CheckReport { failures, notes, stops }
}

/// What `check` reports of one topic file of the records: the judgement
/// `approve` refuses on (`rules::judge_topic_file`), less the naming and
/// heading rules `plan::check_plan` reports for every file of the
/// directory itself (a name, a heading twice in one file, a heading in two
/// files, each a note or a failure by which question copies).
fn topic_file_findings(j: &rules::TopicFileJudge) -> Vec<rules::Finding> {
    rules::judge_topic_file(j)
}

/// The arguments `check` ran with, as the command line names them: what a
/// refusal's move names to run again (per "Does a refusal name the command
/// that clears it?").
fn check_args(staged: bool, message: Option<&Path>, since: Option<&str>) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();
    if staged {
        args.push("--staged".to_string());
    }
    if let Some(m) = message {
        args.push("--message".to_string());
        args.push(m.display().to_string());
    }
    if let Some(s) = since {
        args.push("--since".to_string());
        args.push(s.to_string());
    }
    args
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
            let args: Vec<&str> = args.iter().map(String::as_str).collect();
            out::eprint(&[stopped(msg::not_a_repo("check", &args, Actor::Agent))]);
            return 1;
        }
    };

    // The commit-msg hook: the staged files were judged by the pre-commit
    // hook of the same commit, so only the message is read, for banned
    // words (per "What does the ban scan read?": the staged files and the
    // commit message before every commit). Each finding prints once a commit.
    // A merge is the exception: git runs no pre-commit hook for a merge it
    // commits itself, so at a merge (`MERGE_HEAD` set) the commit-msg hook
    // judges the staged files too, the ledger rows of both branches among
    // them (per "Does check at a merge look for the ledger rows of both
    // branches?").
    let merging = repo::git(&root, &["rev-parse", "-q", "--verify", "MERGE_HEAD"]).is_some();
    if let Some(m) = message.as_deref().filter(|_| !merging) {
        let mut stops: Vec<String> = Vec::new();
        let found = banned::check_banned(&root, false, Some(m), &Source::for_check(false, true), &mut stops);
        let mut lines: Vec<Line> = stops.into_iter().map(stopped).collect();
        lines.extend(found.into_iter().map(Line::Finding));
        out::print(&lines);
        return if lines.is_empty() { 0 } else { 1 };
    }
    let report = build(&root, staged, message.as_deref(), since.as_deref(), None, None);
    let exit_code = if report.failures.is_empty() && report.stops.is_empty() { 0 } else { 1 };
    let mut lines: Vec<Line> = report.stops.into_iter().map(stopped).collect();
    lines.extend(report.failures.into_iter().chain(report.notes).map(Line::Finding));
    out::print(&lines);
    exit_code
}

/// A finding on what `check` was given rather than on the records (no
/// repository, unreadable input), filed under `check`; `message` may carry
/// that name in front already.
pub(super) fn stopped(message: String) -> Line {
    out::refusal(Command::Check, &message)
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
    if !matches!(r.class, rules::Class::Tells | rules::Class::Plan | rules::Class::Parse) {
        return false;
    }
    // Every question the finding is about must still be able to change; a
    // finding about none is about the file as a whole.
    !r.sections.iter().filter_map(|i| rf.sections.get(*i)).any(|s| rules::has_force(rf, s, led))
}
