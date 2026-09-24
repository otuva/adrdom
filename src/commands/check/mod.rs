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
use crate::ledger::Ledger;
use crate::msg::{self, Actor};
use crate::record::{self, RecordFile};
use crate::repo;
use crate::state::{self, rel_path};
use crate::terms as terms_mod;
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
            Err(e) => notes.push(msg::topic_file_unreadable(&f.display().to_string(), e)),
        }
    }

    // Terms minted in memory (E1.3): a terms file that has never been
    // approved carries no ids on disk yet, and `phrase_lookup` only
    // resolves a term that has one. Computed once here, shared by every
    // category below that resolves a `depends:` entry, rather than each
    // reloading/minting its own copy.
    let terms_list: Vec<terms_mod::Term> = state::load_terms(root)
        .map(|tf| terms_mod::minted_copy(&tf).terms)
        .unwrap_or_default();
    let phrase_lookup = terms_mod::phrase_lookup(&terms_list);

    // Every refusal each topic file earns on its own (E1.2, shared with
    // `approve`'s `do_approve_record`), one `Vec` per record in `records`'
    // order, plus the cross-file "answered id missing from every topic
    // file" refusal, computed once for the whole repo. `parse`/`tells`/
    // `plan` pull their failures from these instead of re-deriving the
    // same shape/tells/depends checks `approve` already refuses on; `ids`
    // pulls the missing-answered-id lines the same way.
    let per_record_refusals: Vec<Vec<actions::Refusal>> = records
        .iter()
        .map(|rf| actions::record_refusals(&rf.rel, rf, &terms_list, &phrase_lookup, &led))
        .collect();
    let missing_answered = actions::missing_answered_refusals(&records, &led);

    let mut failures: Vec<String> = Vec::new();
    push_failures(&mut failures, parse::check_parse(&records, &per_record_refusals));
    push_failures(&mut failures, ids::check_ids(&records, &led, &missing_answered));
    push_failures(&mut failures, ledger::check_ledger(&led, &records, root, &ledger_path));
    push_failures(&mut failures, banned::check_banned_ledger(&led, root));
    push_failures(&mut failures, answers::check_answers(&records, &led));
    push_failures(&mut failures, banned::check_banned(root, staged, message));
    push_failures(&mut failures, install::check_install(root));
    let (plan_failures, awaiting) = plan::check_plan(
        root,
        &records,
        &led,
        &per_record_refusals,
        &terms_list,
        &phrase_lookup,
    );
    push_failures(&mut failures, plan_failures);
    let (terms_failures, terms_awaiting, terms_notes) = terms::check_terms(root, &records, &led, staged);
    push_failures(&mut failures, terms_failures);
    let (test_failures, tested_count) = tests::check_tests(root, &records, &led, tests);
    push_failures(&mut failures, test_failures);
    push_failures(&mut failures, tells::check_tells(root, &records, &led, &per_record_refusals));

    notes.extend(moot::check_moot(&records, &led));

    for rf in &records {
        if let Some(tiers) = &rf.tiers {
            // Live counts only (8.8 A): a dead (refused/superseded/
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
    for d in done::check_done(root, &records, &led) {
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
