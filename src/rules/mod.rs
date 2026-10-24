//! Every rule on the records, judged once (per "Does a command refuse what
//! check passes?", fe14991d A): each rule is a function here returning
//! `Finding`s, and every command that acts on a rule reads the same
//! findings. `approve` refuses the findings about the file it approves and
//! the questions it asks, `check` prints them all
//! (a finding in a draft as a note, per "What does check do with a fault on
//! a file awaiting approval?"), and `show` and `plan` name their moves. A
//! rule on the input of one command (the words `ban` takes, the letter and
//! version `answer` takes) stays with that command.
mod depends;
mod judge;
mod removal;
mod terms_file;
mod topic;

pub use depends::{
    cap_finding, depends_line_cap_findings, depends_shape_refusals, derive_finding,
    supersedes_refusals,
};
pub use judge::{
    banned_in_questions, judge_blame_coverage, judge_blame_file, judge_records,
    judge_records_except, judge_removed_topic_file, judge_terms_file, judge_topic_file,
    judge_topic_file_in, judged_terms, no_body_refusals, planning_rules_apply, stale_files,
    struck_asked, struck_asked_ids, terms_file_findings, terms_gate, RepoIndex, TermsGate,
    TopicFileJudge,
};
pub(crate) use removal::latest_approved_version;
pub use removal::{removal_awaiting, removal_holds, RemovalHolds};
pub use terms_file::{terms_banned_hits, terms_refusals};
pub use topic::{
    blame_line_refusals, closed_findings, copying_sections, cross_file_heading_refusals,
    duplicate_heading_refusals, has_force, letter_refusals, missing_answered_refusals,
    missing_answered_refusals_except, own_name_and_heading_refusals, own_name_refusals,
    parse_findings, record_refusals,
};

use crate::msg;
use crate::record::{self, RecordFile};
use crate::repo;
use crate::rule::Rule;
use std::collections::HashSet;

/// What a finding is about.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Subject {
    /// A topic file, by its path.
    TopicFile(String),
    TermsFile,
    /// A blame file, by its path.
    BlameFile(String),
    /// The records as a whole: a ledger id no file holds, for one.
    Records,
    /// The topic files as a directory, by the path of the file the finding
    /// is in: its name, or a heading it holds twice or shares with another
    /// file. `check` reads these off the directory itself, a file that does
    /// not parse included.
    Directory(String),
}

/// The class a finding is filed under inside the tool: what `check` sorts
/// failures from notes by, and what `approve` picks the findings
/// they refuse on by. Never printed: a finding prints its found text and
/// its move, with no tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Class {
    Parse,
    Plan,
    PlanFrozen,
    Tells,
    Answers,
    Ids,
    Terms,
    Blame,
    Banned,
    Ledger,
    Reader,
    Install,
    Tested,
    Untested,
    Brief,
    /// A finding of no class `check` or a command picks it by.
    Other,
}

/// The command whose own input, environment, files or gate a stop is about.
/// A stop prints `<command>: <what>; <move>`; a finding never carries a
/// command's name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Command {
    Check,
    Show,
    Approve,
    Answer,
    Ban,
    Unban,
    Install,
    Uninstall,
    Serve,
    Read,
    Plan,
}

impl Command {
    pub fn as_str(self) -> &'static str {
        match self {
            Command::Check => "check",
            Command::Show => "show",
            Command::Approve => "approve",
            Command::Answer => "answer",
            Command::Ban => "ban",
            Command::Unban => "unban",
            Command::Install => "install",
            Command::Uninstall => "uninstall",
            Command::Serve => "serve",
            Command::Read => "read",
            Command::Plan => "plan",
        }
    }
}

impl std::fmt::Display for Command {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// One rule a thing in the records breaks: the rule, what was found, and
/// the move that clears it. Printed as `<found>; <move>`, with no leading
/// tag, so the same line reads the same under every command.
#[derive(Debug, Clone)]
pub struct Finding {
    pub subject: Subject,
    pub rule: &'static Rule,
    /// The class the tool sorts the finding by; never printed.
    pub class: Class,
    /// What was found, with the place it was found at.
    pub found: String,
    /// The move that clears it, whole (`agent ...`, `owner runs: ...`,
    /// `not yet ...`).
    pub next: msg::Next,
    /// The questions of the file the finding is about, by index; empty for
    /// a finding about the file as a whole (its prose, headings, tiers).
    /// `check` reads it to tell a finding in text that can still change from
    /// one in an asked or answered question.
    pub sections: Vec<usize>,
}

impl Finding {
    /// A finding about the records of no class; `about` names its subject.
    pub fn new(rule: &'static Rule, found: String, next: msg::Next) -> Finding {
        Finding {
            subject: Subject::Records,
            rule,
            class: Class::Other,
            found,
            next,
            sections: Vec::new(),
        }
    }
    /// A finding of `class` from what a message function said.
    pub fn said(class: Class, said: msg::Said) -> Finding {
        Finding::new(said.rule, said.found, said.next).class(class)
    }
    pub fn class(mut self, class: Class) -> Finding {
        self.class = class;
        self
    }
    pub fn about(mut self, subject: Subject) -> Finding {
        self.subject = subject;
        self
    }
    pub fn in_section(mut self, index: usize) -> Finding {
        self.sections.push(index);
        self
    }
    /// Whether this finding keeps the reader from reading the file's bodies:
    /// one the agent clears without the reader (a word over the limit, a
    /// banned word, a depends line over its bounds, a tell), other than the
    /// planning's own (a step without its record, a neighbour on no line),
    /// which the reading is there to clear. A finding whose move is the
    /// owner's or nobody's yet stops nothing: the reading goes on beside it.
    pub fn blocks_the_reader(&self) -> bool {
        self.next.is_agents()
            && self.rule.name != crate::rule::PLAN_STEP_HAS_ITS_RECORD.name
            && self.rule.name != crate::rule::NEIGHBOUR_IS_ON_A_LINE.name
    }
    pub fn in_sections(mut self, indexes: Vec<usize>) -> Finding {
        self.sections = indexes;
        self
    }
    /// The finding as one line: what was found and the move that clears it. The one rendering every command prints.
    pub fn text(&self) -> String {
        msg::finding_line(&self.found, &self.next)
    }
}

/// The one order findings print in, under every command and on the page: by
/// the file the finding is about (the records as a whole first), then the
/// first question of the file it is about (the file as a whole first), then
/// the rule's place in `rule::ALL`, then its text.
pub fn finding_order(a: &Finding, b: &Finding) -> std::cmp::Ordering {
    fn key(f: &Finding) -> (String, usize, usize, String) {
        let file = match &f.subject {
            Subject::Records => String::new(),
            Subject::TermsFile => repo::TERMS_PATH.to_string(),
            Subject::TopicFile(p) | Subject::BlameFile(p) | Subject::Directory(p) => p.clone(),
        };
        let question = f.sections.iter().min().map_or(0, |i| i + 1);
        let rule = crate::rule::ALL
            .iter()
            .position(|r| r.name == f.rule.name)
            .unwrap_or(usize::MAX);
        (file, question, rule, f.text())
    }
    key(a).cmp(&key(b))
}

/// Tags each finding of `fs` as about the topic file `rel`, a finding about
/// the records left as it is.
fn about_topic_file(fs: Vec<Finding>, rel: &str) -> Vec<Finding> {
    fs.into_iter()
        .map(|f| {
            if f.subject == Subject::Records {
                f.about(Subject::TopicFile(rel.to_string()))
            } else {
                f
            }
        })
        .collect()
}

/// Strings of one rule, each a finding about the topic file `rel`.
fn found(class: Class, rel: &str, messages: Vec<msg::Said>) -> Vec<Finding> {
    messages
        .into_iter()
        .map(|m| Finding::said(class, m).about(Subject::TopicFile(rel.to_string())))
        .collect()
}

/// Every identifier the topic files `all` hold: the one set of the
/// questions in the files every reader of "held by no file" reads.
pub fn held_ids(all: &[RecordFile]) -> HashSet<&str> {
    all.iter()
        .flat_map(|rf| rf.sections.iter().filter_map(|s| s.id.as_deref()))
        .collect()
}

/// A path of the shape of a topic file: `adrdom/decisions/<name>.md`.
pub(crate) fn is_topic_file_path(rel: &str) -> bool {
    // Any `.md` under the directory, nested too: the files the loader,
    // show, check, ask and approve take (`repo::list_decision_files_via`).
    rel.strip_prefix(&format!("{}/", repo::DECISIONS_DIR))
        .is_some_and(|name| name.ends_with(".md"))
}

/// A path of the shape of a blame file: `adrdom/blame/<name>.md`, nested
/// paths too (`blame::Index` lists the same files).
pub(crate) fn is_blame_file_path(rel: &str) -> bool {
    rel.strip_prefix(&format!("{}/", repo::BLAME_DIR))
        .is_some_and(|name| name.ends_with(".md"))
}

/// Whether `rel` (a path from the repository root) names a file approve
/// takes: a topic file under adrdom/decisions, adrdom/terms.md, or a blame
/// file under adrdom/blame.
pub fn approve_takes(rel: &str) -> bool {
    rel == repo::TERMS_PATH || is_topic_file_path(rel) || is_blame_file_path(rel)
}

/// The plan hash of a file with no question; the same value the context
/// hash of a file with no prose has. What approving a file's removal writes.
pub(crate) fn empty_plan_hash() -> String {
    record::derive_file_hash(&[])
}
