mod answers;
mod banned;
mod done;
mod ids;
mod install;
mod ledger;
mod moot;
mod parse;
mod plan;
mod terms;
mod tests;

pub(crate) use banned::collect_targets;
pub use tests::test_index;

use crate::ledger::Ledger;
use crate::record::{self, RecordFile};
use crate::repo;
use crate::state::rel_path;
use crate::view;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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
    let ledger_path = root.join("adrdom/state/ledger.tsv");
    let led = crate::ledger::read(&ledger_path);

    let files = repo::list_decision_files(root);
    let mut records = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    for f in &files {
        match record::load_in(root, f) {
            Ok(rf) => records.push(rf),
            Err(e) => notes.push(format!("parse: {}: cannot read: {}", f.display(), e)),
        }
    }

    let mut failures: Vec<String> = Vec::new();
    push_failures(&mut failures, parse::check_parse(&records));
    push_failures(&mut failures, ids::check_ids(&records, &led));
    push_failures(&mut failures, ledger::check_ledger(&led, &records, root, &ledger_path));
    push_failures(&mut failures, banned::check_banned_ledger(&led, root));
    push_failures(&mut failures, answers::check_answers(&records, &led));
    push_failures(&mut failures, banned::check_banned(root, staged, message));
    push_failures(&mut failures, install::check_install(root));
    let (plan_failures, awaiting) = plan::check_plan(root, &records, &led);
    push_failures(&mut failures, plan_failures);
    let (terms_failures, terms_awaiting, terms_notes) = terms::check_terms(root, &records, &led, staged);
    push_failures(&mut failures, terms_failures);
    let (test_failures, tested_count) = tests::check_tests(root, &records, &led, tests);
    push_failures(&mut failures, test_failures);

    notes.extend(moot::check_moot(&records, &led));

    for rf in &records {
        if let Some(tiers) = &rf.tiers {
            let sizes = record::tier_sizes(tiers)
                .into_iter()
                .map(|(t, n)| format!("{} ({})", t, n))
                .collect::<Vec<_>>()
                .join(" ");
            notes.push(format!("tiers: {}: {}", rf.rel, sizes));
        }
    }

    if !awaiting.is_empty() {
        notes.push(format!(
            "plan: {} file(s) awaiting approval: {}",
            awaiting.len(),
            awaiting.join(", ")
        ));
    }
    if terms_awaiting {
        notes.push("terms: awaiting approval".to_string());
    }
    notes.extend(terms_notes);
    for d in done::check_done(root, &records, &led) {
        notes.push(format!("done: {}", d));
    }
    notes.push(format!("tested: {}", tested_count));

    view::CheckReport { failures, notes }
}

pub fn run(staged: bool, message: Option<PathBuf>) -> i32 {
    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            eprintln!("check: not inside a git repository");
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
/// and likewise for context/`context`.
fn file_approval(root: &Path, rf: &RecordFile, led: &Ledger) -> (String, bool, bool) {
    let rel = rel_path(root, &rf.path);
    let plan_h = record::plan_hash(rf);
    let ctx_h = record::context_hash(rf);
    let plan_ok = led
        .latest_for_path("approved", &rel)
        .map(|r| r.hash == plan_h)
        .unwrap_or(false);
    let ctx_ok = led
        .latest_for_path("context", &rel)
        .map(|r| r.hash == ctx_h)
        .unwrap_or(false);
    (rel, plan_ok, ctx_ok)
}
