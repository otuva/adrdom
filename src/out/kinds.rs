//! The closed-list statuses a line carries and the events a writing command wrote.
use crate::ledger::Row;
use crate::status::{FileStatus, QuestionStatus, TermStatus};
use std::collections::HashMap;

/// A status from one of the closed lists of `status`, with what the name
/// carries beside it: the picked letter of an answered question, the
/// version of an asked one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Question {
        status: QuestionStatus,
        letter: Option<char>,
        version: Option<String>,
    },
    File(FileStatus),
    Term(TermStatus),
}

impl Status {
    /// The status's name: the one `name()` of its list, with the letter of
    /// an answered question and the version of an asked one.
    pub fn name(&self) -> String {
        match self {
            Status::Question {
                status,
                letter,
                version,
            } => match version {
                Some(v) => format!("{} @{}", status.name_with(*letter), v),
                None => status.name_with(*letter),
            },
            Status::File(s) => s.name().to_string(),
            Status::Term(s) => s.name().to_string(),
        }
    }
}

/// One event a writing command wrote: a ledger row, or a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// The plan of a topic file approved (its file row).
    PlanApproved {
        path: String,
    },
    /// One question's text approved (its own row).
    QuestionApproved {
        heading: String,
    },
    /// The prose of a topic file approved.
    ProseApproved {
        path: String,
    },
    TermsApproved,
    TermApproved {
        name: String,
    },
    TermRemoved {
        name: String,
    },
    BlameApproved {
        path: String,
    },
    Asked {
        heading: String,
    },
    /// A question asked while one standing on it carries an answer set.
    Assumes {
        dependent: String,
        target: String,
    },
    /// `sentence` is the picked option's sentence when the caller knows it
    /// (`answer` does; a row alone does not carry it).
    Answered {
        heading: String,
        letter: String,
        sentence: String,
    },
    Refused {
        heading: String,
    },
    Superseded {
        heading: String,
    },
    Withdrawn {
        heading: String,
    },
    Dropped {
        heading: String,
    },
    Banned {
        term: Option<String>,
    },
    Unbanned,
    FileWritten {
        path: String,
    },
    FileRemoved {
        path: String,
    },
    /// A key of git's configuration set to `value` (install's
    /// `core.hooksPath`).
    ConfigSet {
        key: String,
        value: String,
    },
    /// A key of git's configuration unset.
    ConfigUnset {
        key: String,
    },
}

impl Event {
    /// The event a ledger row records. `names` holds the heading of every
    /// question and the name of every term by identifier; an identifier it
    /// does not hold is printed as it is.
    pub fn from_row(row: &Row, names: &HashMap<String, String>) -> Option<Event> {
        let name = |id: &str| names.get(id).cloned().unwrap_or_else(|| id.to_string());
        Some(match row.event.as_str() {
            "approved" if row.id == "-" => Event::PlanApproved {
                path: row.detail.clone(),
            },
            "approved" => Event::QuestionApproved {
                heading: name(&row.id),
            },
            "context" => Event::ProseApproved {
                path: row.detail.clone(),
            },
            "terms" => Event::TermsApproved,
            "term" if row.hash == crate::state::TERM_REMOVED => Event::TermRemoved {
                name: row.detail.clone(),
            },
            "term" => Event::TermApproved {
                name: row.detail.clone(),
            },
            "blame" => Event::BlameApproved {
                path: row.detail.clone(),
            },
            "asked" => Event::Asked {
                heading: row.detail.clone(),
            },
            "assumes" => Event::Assumes {
                dependent: name(&row.detail),
                target: name(&row.id),
            },
            "answered" => Event::Answered {
                heading: name(&row.id),
                letter: row.detail.clone(),
                sentence: String::new(),
            },
            "refused" => Event::Refused {
                heading: row.detail.clone(),
            },
            "superseded" => Event::Superseded {
                heading: name(&row.id),
            },
            "withdrawn" => Event::Withdrawn {
                heading: row.detail.clone(),
            },
            "dropped" => Event::Dropped {
                heading: name(&row.id),
            },
            "banned" => Event::Banned {
                term: (!row.detail.is_empty()).then(|| name(&row.detail)),
            },
            "unbanned" => Event::Unbanned,
            _ => return None,
        })
    }

    pub fn text(&self) -> String {
        match self {
            Event::PlanApproved { path } => format!("approved: plan of {}", path),
            Event::QuestionApproved { heading } => format!("approved: \"{}\"", heading),
            Event::ProseApproved { path } => format!("approved: prose of {}", path),
            Event::TermsApproved => format!("approved: {}", crate::repo::TERMS_PATH),
            Event::TermApproved { name } => format!("approved: term \"{}\"", name),
            Event::TermRemoved { name } => format!("removed: term \"{}\"", name),
            Event::BlameApproved { path } => format!("approved: {}", path),
            Event::Asked { heading } => format!("asked: \"{}\"", heading),
            Event::Assumes { dependent, target } => {
                format!(
                    "assumes: \"{}\" stands on \"{}\" under its answer set",
                    dependent, target
                )
            }
            Event::Answered {
                heading,
                letter,
                sentence,
            } if sentence.is_empty() => {
                format!("answered: \"{}\" {}", heading, letter)
            }
            Event::Answered {
                heading,
                letter,
                sentence,
            } => {
                format!("answered: \"{}\" {}: {}", heading, letter, sentence)
            }
            Event::Refused { heading } => format!("refused: \"{}\"", heading),
            Event::Superseded { heading } => format!("superseded: \"{}\"", heading),
            Event::Withdrawn { heading } => format!("withdrawn: \"{}\"", heading),
            Event::Dropped { heading } => format!("dropped: \"{}\"", heading),
            Event::Banned { term: Some(t) } => format!("banned: a word of term \"{}\"", t),
            Event::Banned { term: None } => "banned: a word".to_string(),
            Event::Unbanned => "unbanned: a word".to_string(),
            Event::FileWritten { path } => format!("written: {}", path),
            Event::FileRemoved { path } => format!("removed: {}", path),
            Event::ConfigSet { key, value } => format!("set: {} {}", key, value),
            Event::ConfigUnset { key } => format!("unset: {}", key),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(event: &str, id: &str, hash: &str, detail: &str) -> Row {
        Row {
            time: String::new(),
            event: event.to_string(),
            id: id.to_string(),
            hash: hash.to_string(),
            detail: detail.to_string(),
            version: String::new(),
        }
    }

    #[test]
    fn every_ledger_event_a_command_writes_reads_as_an_event() {
        let names: HashMap<String, String> = [("id-1".to_string(), "Does it?".to_string())].into();
        for event in crate::ledger::EVENTS {
            let r = row(event, "id-1", "h", "d");
            assert!(
                Event::from_row(&r, &names).is_some(),
                "no event for a {} row",
                event
            );
        }
        let answered = Event::from_row(&row("answered", "id-1", "h", "A"), &names).unwrap();
        assert_eq!(answered.text(), "answered: \"Does it?\" A");
        let plan =
            Event::from_row(&row("approved", "-", "h", "adrdom/decisions/x.md"), &names).unwrap();
        assert_eq!(plan.text(), "approved: plan of adrdom/decisions/x.md");
    }
}
