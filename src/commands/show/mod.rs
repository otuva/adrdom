//! `adr show`: what the records hold and every move. With no file, every
//! current decision and every move (per "What does show print when no file
//! is named?", c5e10ae8 A). With a file approve takes -- a topic file, the
//! terms file or a blame file -- every question or entry with its
//! status (a blame file: its origin and each row as written), every depends
//! and supersedes line and the body of an asked question, with what the file
//! holds beyond its latest approval marked (per "Does show print what a file
//! holds past its latest approval?", 91f9652a A), then the moves of that
//! file. What it computes from the records comes under a flag (per
//! "Does a command print a computed view under a flag?", cdc905a2 A):
//! `--neighbours`, `--dependents`, `--unrelated` and `--questions`, `--reader`
//! and `--superseded`. The reference lines of an
//! answered question are one of them: printed under `--dependents` (per "Does
//! show print the reference lines of a question that is answered?", d1d6fed2 A).
//! What the next move on the file reads is in the plain listing, the rest
//! under a flag (per "What does show print for a file under adrdom?",
//! b0acc95b A): the option picked prints under an answered question, what
//! the build move reads; a superseded question, which no move reads, prints
//! under `--superseded` with what replaced it. The reader's word
//! `inconsistent`, `same` or `closes` with its letter on a neighbour comes
//! under a question not yet answered in the plain listing, every word under
//! `--reader` (per "What does show print of a reader's word on a question?",
//! 6e72f34e A). Every line goes through
//! `out` (per "What does a command print when it writes nothing?", 1af76f91 A).
mod file;
mod findings;
mod listing;
mod topic;

pub(crate) use file::{
    file_form, file_form_lines, neighbours_map, neighbours_of, topic_file_status_line,
};
pub(crate) use findings::{terms_report, topic_file_report_judged};
pub(crate) use listing::{
    decision_lines, listed_report_judged, rest, summary, unreadable_topic_lines,
};
pub(crate) use topic::{question_status_of, term_dependents};

use file::whole;

use crate::blame;
use crate::ledger::{self, Ledger};
use crate::msg::{self, Actor};
use crate::out::{self, Line};
use crate::record::{self, RecordFile};
use crate::repo;
use crate::rules::Command;
use crate::rules::{self};
use crate::state;
use crate::terms::{self, Term, TermsFile};
use crate::view;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// The computed views `show` prints under a flag.
#[derive(Debug, Clone, Copy, Default)]
pub struct Views {
    /// Each neighbour of a question not yet answered, with the reader's word
    /// and the agent's line.
    pub neighbours: bool,
    /// What stands on each question or term entry.
    pub dependents: bool,
    /// Each superseded question of a topic file, with what replaced it.
    pub superseded: bool,
    /// The `unrelated:` line of each question, marked when changed.
    pub unrelated: bool,
    /// For a blame file: how many of its decision rows a question names and
    /// how many none names (per "What does check do with a decision row
    /// that no question names?", ee27fd4c A).
    pub questions: bool,
    /// Every word the reader gave on every neighbour of each question, with
    /// its reason.
    pub reader: bool,
}

/// Everything `show` reads, loaded once; the page reads its lines from the
/// same (per "Does the ui show what the terminal prints?", 7857d633 A).
pub(crate) struct Records {
    pub(crate) root: PathBuf,
    pub(crate) led: Ledger,
    pub(crate) terms_file: Option<TermsFile>,
    /// The error of an adrdom/terms.md that exists and cannot be read:
    /// named, never skipped in silence.
    terms_unreadable: Option<String>,
    pub(crate) terms_list: Vec<Term>,
    pub(crate) topic_files: Vec<RecordFile>,
    pub(crate) unreadable_topic_files: Vec<(String, String)>,
    blame: blame::Index,
    pub(crate) unreadable_blame_files: Vec<(String, String)>,
    pub(crate) tests: HashMap<String, Vec<String>>,
    /// Every question's heading, every term's name and every blame file's
    /// name by its identifier: what turns an identifier on a line back into
    /// words.
    pub(crate) names: HashMap<String, String>,
    /// What the judgement of every topic file reads of the whole
    /// repository, read the first time a file is judged and kept for the
    /// rest (`rules::RepoIndex`).
    index: std::cell::OnceCell<rules::RepoIndex>,
    /// The terms file with an identifier on every entry
    /// (`terms::minted_copy`), minted the first time it is needed: the one
    /// list the terms judgement reads, as in `check`.
    minted_terms: std::cell::OnceCell<Option<TermsFile>>,
}

impl Records {
    pub(crate) fn load(root: PathBuf) -> Records {
        let led = ledger::read(&root.join(repo::LEDGER_PATH));
        let (terms_file, terms_unreadable) = match state::load_terms_checked(&root) {
            state::TermsLoad::Ok(tf) => (Some(tf), None),
            state::TermsLoad::LoadError(e) => (None, Some(e)),
            state::TermsLoad::Missing => (None, None),
        };
        // The terms every topic file is judged against, as check judges
        // them: one minted copy, so an entry with no id yet carries one
        // identifier.
        let minted = terms_file.as_ref().map(terms::minted_copy);
        let terms_list: Vec<Term> = minted
            .as_ref()
            .map(|tf| tf.terms.clone())
            .unwrap_or_default();
        let (topic_files, unreadable) = record::load_all_via(&root, &repo::Source::WorkingTree);
        let unreadable_topic_files = unreadable
            .into_iter()
            .map(|(f, e)| (state::rel_path(&root, &f), e.to_string()))
            .collect();
        let (blame_files, unreadable_blame_files) = blame::load_all_with_errors(&root);
        let blame = blame::Index::of(blame_files, &led);
        let tests = crate::commands::check::test_index(&root, &repo::Source::WorkingTree);
        let mut names: HashMap<String, String> = terms_list
            .iter()
            .filter_map(|t| Some((t.id.clone()?, t.name.clone())))
            .collect();
        // A question no topic file holds any more (struck, or its file
        // removed) by the heading its latest `asked` row carries; one a
        // topic file holds by its heading there.
        for row in led
            .rows
            .iter()
            .filter(|r| r.event == "asked" && !r.detail.is_empty())
        {
            names.insert(row.id.clone(), row.detail.clone());
        }
        names.extend(view::heading_texts(&topic_files));
        // A blame file by the name its path gives, under the identifier its
        // id line holds: what turns the identifier of a `blame:` line back
        // into the name approve turned (a copy of a file's id names nothing,
        // and no blame file's name replaces a question's or a term's).
        for e in blame.entries() {
            let Some(id) = e.id() else { continue };
            if blame.keeper_of(id).is_some_and(|k| k.rel() == e.rel()) {
                names
                    .entry(id.to_ascii_lowercase())
                    .or_insert_with(|| e.name.clone());
            }
        }
        Records {
            root,
            led,
            terms_file,
            terms_unreadable,
            terms_list,
            topic_files,
            unreadable_topic_files,
            blame,
            unreadable_blame_files,
            tests,
            names,
            index: std::cell::OnceCell::new(),
            minted_terms: std::cell::OnceCell::from(minted),
        }
    }

    /// The finding on an adrdom/terms.md that exists and cannot be read.
    pub(crate) fn terms_unreadable_finding(&self) -> Option<Line> {
        let e = self.terms_unreadable.as_ref()?;
        Some(out::refusal(
            Command::Show,
            &msg::file_unreadable(repo::TERMS_PATH, e),
        ))
    }

    pub(crate) fn minted_terms(&self) -> Option<&TermsFile> {
        self.minted_terms
            .get_or_init(|| self.terms_file.as_ref().map(terms::minted_copy))
            .as_ref()
    }

    pub(crate) fn index(&self) -> &rules::RepoIndex {
        self.index.get_or_init(|| {
            rules::RepoIndex::new_with_blame(
                &self.root,
                &self.topic_files,
                &self.terms_list,
                &self.led,
                &repo::Source::WorkingTree,
                self.blame.clone(),
            )
        })
    }

    fn present_ids(&self) -> HashSet<String> {
        rules::held_ids(&self.topic_files)
            .into_iter()
            .map(str::to_string)
            .collect()
    }

    pub(crate) fn removed_topic_files(&self) -> Vec<String> {
        rules::removal_awaiting(&self.led, &|rel| self.root.join(rel).exists())
    }
}

/// The arguments `show` ran with, as the command line names them: what a
/// refusal's move names to run again (per "Does a refusal name the command
/// that clears it?").
fn show_args(record_arg: Option<&std::path::Path>, all: bool, views: &Views) -> Vec<String> {
    let mut args: Vec<String> = Vec::new();
    if let Some(p) = record_arg {
        args.push(p.display().to_string());
    }
    if all {
        args.push("--all".to_string());
    }
    if views.neighbours {
        args.push("--neighbours".to_string());
    }
    if views.dependents {
        args.push("--dependents".to_string());
    }
    if views.unrelated {
        args.push("--unrelated".to_string());
    }
    if views.questions {
        args.push("--questions".to_string());
    }
    if views.reader {
        args.push("--reader".to_string());
    }
    if views.superseded {
        args.push("--superseded".to_string());
    }
    args
}

pub fn run(record_arg: Option<PathBuf>, all: bool, views: Views) -> i32 {
    let Some(root) = repo::find_root() else {
        let args = show_args(record_arg.as_deref(), all, &views);
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        out::eprint(&[out::refusal(
            Command::Show,
            &msg::not_a_repo("show", &args, Actor::Agent),
        )]);
        return 1;
    };
    let r = Records::load(root);
    let lines = match &record_arg {
        Some(p) => match file_form(
            &r,
            &state::given_rel(&r.root, p).unwrap_or_else(|| p.display().to_string()),
            views,
        ) {
            Ok(lines) => lines,
            Err(refusal) => {
                out::eprint(&[out::refusal(Command::Show, &refusal)]);
                return 1;
            }
        },
        None if all => whole(&r, views),
        None => summary(&r),
    };
    out::print(&lines);
    0
}
