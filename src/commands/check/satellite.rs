//! `check` in a satellite: the satellite's own files judged against the
//! primary's record (per "What does check in a satellite read the record
//! from?"). The record is read from the primary's working tree always; the
//! satellite's files through the run's source. The primary's topic files,
//! terms file, ledger, reader rows and blame files are the primary's own
//! check to judge, not this one.
use super::{banned, brief, install, planner, tests, Failure};
use crate::ledger;
use crate::msg;
use crate::record;
use crate::repo::{self, Source};
use crate::rules::{self, Class};
use crate::satellite::Place;
use crate::state;
use crate::terms;
use crate::view;
use std::collections::HashMap;
use std::path::Path;

/// The report of the satellite at `place`. `staged` and `message` are the
/// hook modes of `run`; `tests` is the ties index of the satellite when the
/// caller holds one already (`show`), else it is built here.
pub(crate) fn report(
    place: &Place,
    staged: bool,
    message: Option<&Path>,
    tests: Option<&HashMap<String, Vec<String>>>,
) -> view::CheckReport {
    let (code, record_root) = (place.code.as_path(), place.record.as_path());
    let source = Source::for_check(staged, message.is_some());
    let wt = Source::WorkingTree;
    let led = ledger::parse(&wt.read(record_root, repo::LEDGER_PATH).unwrap_or_default());
    let (records, _unreadable) = record::load_all_via(record_root, &wt);
    let tf = state::load_terms_via(record_root, &wt);
    let terms_list: Vec<terms::Term> = tf
        .as_ref()
        .map(|t| terms::minted_copy(t).terms)
        .unwrap_or_default();

    let mut failures: Vec<Failure> = Vec::new();
    let mut notes: Vec<rules::Finding> = Vec::new();
    let mut stops: Vec<String> = Vec::new();

    failures.extend(install::check_install_in(code, &source, true));
    failures.extend(brief_copy(code, record_root, &source));
    if let Some(tf) = &tf {
        failures.extend(terms_brief_copy(code, &source, tf, &led));
    }
    failures.extend(banned::check_banned_in(
        record_root,
        &wt,
        code,
        staged,
        message,
        &mut stops,
    ));
    if staged || message.is_some() {
        failures.extend(planner::check_commit(code, record_root));
    }
    let role = crate::session::role(record_root);
    let (brief_failures, brief_notes) =
        brief::check_brief(code, &source, &records, &terms_list, &led, role);
    failures.extend(brief_failures);
    notes.extend(brief_notes);
    let owned;
    let index = match tests {
        Some(t) => t,
        None => {
            owned = tests::test_index(code, &source);
            &owned
        }
    };
    notes.extend(tests::check_tests(
        record_root,
        &records,
        &led,
        Some(index),
        &source,
        role,
        true,
    ));
    view::CheckReport {
        failures,
        notes,
        stops,
        moves: Vec::new(),
    }
}

/// Whether `got` holds the same lines as `want`, as the other file
/// comparisons of check read them.
fn same(got: &str, want: &str) -> bool {
    crate::text::lines_match_trimmed(
        &got.lines().collect::<Vec<_>>(),
        &want.lines().collect::<Vec<_>>(),
    )
}

/// The brief copy (per "Does check hold a satellite's brief to the
/// primary's?"): the satellite's `adrdom/state/brief.md` against the
/// primary's. At a commit with only the working tree right, the file is
/// named as not staged.
fn brief_copy(code: &Path, primary: &Path, source: &Source) -> Vec<Failure> {
    let Some(want) = Source::WorkingTree.read(primary, repo::BRIEF_PATH) else {
        return Vec::new();
    };
    let ok = |s: &Source| {
        s.read(code, repo::BRIEF_PATH)
            .is_some_and(|got| same(&got, &want))
    };
    if ok(source) {
        return Vec::new();
    }
    let said = if *source == Source::Index && ok(&Source::WorkingTree) {
        msg::install_file_not_staged(repo::BRIEF_PATH)
    } else if source.exists(code, repo::BRIEF_PATH) {
        msg::satellite_brief_differs()
    } else {
        msg::satellite_brief_missing()
    };
    vec![Failure::said(Class::Install, said)]
}

/// The terms brief copy: the satellite's terms brief against the text built
/// from the primary's terms file, once that file is approved with every id
/// (the gate `check_terms` holds the primary's own terms brief to).
fn terms_brief_copy(
    code: &Path,
    source: &Source,
    tf: &terms::TermsFile,
    led: &ledger::Ledger,
) -> Vec<Failure> {
    if !state::terms_approved(led, tf) || !tf.missing_ids().is_empty() {
        return Vec::new();
    }
    let want = terms::brief_text(tf);
    let ok = |s: &Source| {
        s.read(code, repo::TERMS_BRIEF_PATH)
            .is_some_and(|got| same(&got, &want))
    };
    if ok(source) {
        return Vec::new();
    }
    let said = if *source == Source::Index && ok(&Source::WorkingTree) {
        msg::terms_brief_not_staged()
    } else if source.exists(code, repo::TERMS_BRIEF_PATH) {
        msg::satellite_terms_brief_differs()
    } else {
        msg::satellite_terms_brief_missing()
    };
    vec![Failure::said(Class::Terms, said)]
}
