//! How the ledger and the terms file bear on each other: retired and removed terms, waiting entries, and
//! the questions and decisions that depend on a term.
use super::{is_current, terms_approved};
use crate::ledger::Ledger;
use crate::record::{self, RecordFile};
use crate::terms::{self, TermsFile};

/// One current decision's, or one non-retired term's, declared use of a
/// term: what to print (`label`, always the quoted form used everywhere a
/// dependent is printed) and, for a decision, its own id -- named so a
/// caller that must supersede it to clear a refusal has something to point
/// at (a term has nothing to supersede, so `decision_id` is `None` for
/// one).
pub enum DependentSource {
    Decision {
        rel: String,
        heading: String,
        id: String,
    },
    Term {
        name: String,
    },
    /// An asked question (latest ledger state `asked`), by its terms line.
    Asked {
        rel: String,
        heading: String,
    },
    /// A heading of an approved plan with an id and no ledger state yet,
    /// by its heading text.
    PlanHeading {
        rel: String,
        heading: String,
    },
}

impl DependentSource {
    /// `"<rel>: \"<heading>\""` for a decision, `"term \"<name>\""` for a
    /// term -- the one label format used everywhere a dependent is printed.
    pub fn label(&self) -> String {
        match self {
            DependentSource::Decision { rel, heading, .. } => format!("{}: \"{}\"", rel, heading),
            DependentSource::Term { name } => format!("term \"{}\"", name),
            DependentSource::Asked { rel, heading }
            | DependentSource::PlanHeading { rel, heading } => {
                format!("{}: \"{}\"", rel, heading)
            }
        }
    }

    /// The id and heading of a current decision dependent.
    pub fn decision_ref(&self) -> Option<(&str, &str)> {
        match self {
            DependentSource::Decision { id, heading, .. } => Some((id, heading)),
            _ => None,
        }
    }

    pub fn decision_id(&self) -> Option<&str> {
        match self {
            DependentSource::Decision { id, .. } => Some(id),
            _ => None,
        }
    }
}

/// Whether `t`, a retired entry, still has its retirement to record: the
/// ledger's latest `term` row for its id is missing or holds another hash
/// (the hash covers the retired flag, so an approved retire matches).
pub fn retire_unrecorded(led: &Ledger, t: &terms::Term) -> bool {
    match &t.id {
        Some(id) => led
            .latest_for_id("term", id)
            .map_or(true, |r| r.hash != terms::term_hash(t)),
        None => true,
    }
}

/// Whether the terms approve of `tf` retires or removes a term: an entry
/// with the retired flag whose retirement is not recorded yet, or an id
/// with a `term` row and no entry in `tf`.
pub fn approve_retires_or_removes(led: &Ledger, tf: &TermsFile) -> bool {
    if tf
        .terms
        .iter()
        .any(|t| t.retired && retire_unrecorded(led, t))
    {
        return true;
    }
    !removed_terms(led, tf).is_empty()
}

/// The hash of a `term` row that records a removal: the row's detail holds
/// the removed entry's name.
pub const TERM_REMOVED: &str = "-";

/// The entries the terms approve of `tf` removes: an id whose latest `term`
/// row is not a removal and which `tf` no longer holds, as (id, name).
pub fn removed_terms(led: &Ledger, tf: &TermsFile) -> Vec<(String, String)> {
    let mut seen: Vec<&str> = Vec::new();
    let mut out = Vec::new();
    for r in led.rows.iter().filter(|r| r.event == "term") {
        if seen.contains(&r.id.as_str()) {
            continue;
        }
        seen.push(&r.id);
        let latest = led.latest_for_id("term", &r.id).unwrap_or(r);
        if latest.hash != TERM_REMOVED
            && !tf
                .terms
                .iter()
                .any(|t| t.id.as_deref() == Some(r.id.as_str()))
        {
            out.push((r.id.clone(), latest.detail.clone()));
        }
    }
    out
}

/// The entries of a terms edit that wait for the owner's approve, each by
/// name with its status, new, changed or removed, against the latest `term`
/// rows (per "What does approve take for a terms file holding several
/// entries that wait?", da0dc8b1 A: approve takes one of these names per
/// run): empty while the file equals its latest approval. A ledger that
/// holds `terms` rows and no `term` rows yet (the catch-up approve) has no
/// baseline per entry, so an entry with no row is not counted there.
pub fn waiting_terms(led: &Ledger, tf: &TermsFile) -> Vec<(String, crate::status::TermStatus)> {
    if terms_approved(led, tf) {
        return Vec::new();
    }
    use crate::status::TermStatus;
    let mut out: Vec<(String, TermStatus)> = tf
        .terms
        .iter()
        .map(|t| (t.name.clone(), crate::status::term_status(led, tf, t)))
        .filter(|(_, st)| matches!(st, TermStatus::New | TermStatus::Changed))
        .collect();
    out.extend(
        removed_terms(led, tf)
            .into_iter()
            .map(|(_, name)| (name, TermStatus::Removed)),
    );
    out
}

/// The asked questions, and the not-yet-asked headings of approved plans,
/// that name a term of `tf` (retired ones included): the dependents a
/// term's retirement or removal must also clear, beyond the current
/// decisions `dependent_entries` reads (per "What holds a retire of a
/// term?"). An asked question is read by its frozen terms line and by the
/// term ids on its `depends:` line, an approved heading by the same ids; a heading with an id and no ledger state at all, in
/// a file whose rel is in `approved_plans`, by its heading text resolved
/// against every term with the retired flag ignored, a retired term counted
/// only while its retirement has no matching `term` row (a word a current
/// term also owns keeps resolving to the current term). Refused,
/// superseded, withdrawn and answered questions are never returned here.
pub fn unsettled_entries(
    records: &[RecordFile],
    led: &Ledger,
    tf: &TermsFile,
    approved_plans: &std::collections::HashSet<String>,
) -> Vec<DependentEntry> {
    let current: Vec<terms::Term> = tf.terms.iter().filter(|t| !t.retired).cloned().collect();
    let retired: Vec<terms::Term> = tf
        .terms
        .iter()
        .filter(|t| t.retired)
        // Only a retire not yet recorded: the `term` row's hash covers the
        // retired flag, so an approved retire has a matching row and a
        // heading written after it is not a dependent of this approve.
        .filter(|t| retire_unrecorded(led, t))
        .cloned()
        .map(|mut t| {
            t.retired = false;
            t
        })
        .collect();
    let mut out = Vec::new();
    for rf in records {
        for sec in &rf.sections {
            let Some(id) = &sec.id else { continue };
            match led.latest_state(id).map(|r| r.event.as_str()) {
                Some("asked") => {
                    let pairs: Vec<(String, String)> = sec
                        .terms_line
                        .and_then(|tl| terms::parse_terms_line(rf.lines[tl].trim_end()))
                        .unwrap_or_default();
                    let dep_pairs = depends_term_pairs(sec, tf, &pairs);
                    if !pairs.is_empty() {
                        out.push(DependentEntry {
                            source: DependentSource::Asked {
                                rel: rf.rel.clone(),
                                heading: sec.heading_text.clone(),
                            },
                            pairs: pairs.clone(),
                            from_word: true,
                        });
                    }
                    for p in dep_pairs {
                        out.push(DependentEntry {
                            source: DependentSource::Asked {
                                rel: rf.rel.clone(),
                                heading: sec.heading_text.clone(),
                            },
                            pairs: vec![p],
                            from_word: false,
                        });
                    }
                }
                None if approved_plans.contains(&rf.rel) => {
                    let mut pairs = terms::resolve(&sec.heading_text, &current);
                    for p in terms::resolve(&sec.heading_text, &retired) {
                        if !pairs.iter().any(|(w, _)| *w == p.0) {
                            pairs.push(p);
                        }
                    }
                    let dep_pairs = depends_term_pairs(sec, tf, &pairs);
                    if !pairs.is_empty() {
                        out.push(DependentEntry {
                            source: DependentSource::PlanHeading {
                                rel: rf.rel.clone(),
                                heading: sec.heading_text.clone(),
                            },
                            pairs,
                            from_word: true,
                        });
                    }
                    for p in dep_pairs {
                        out.push(DependentEntry {
                            source: DependentSource::PlanHeading {
                                rel: rf.rel.clone(),
                                heading: sec.heading_text.clone(),
                            },
                            pairs: vec![p],
                            from_word: false,
                        });
                    }
                }
                _ => {}
            }
        }
    }
    out
}

/// The terms a section's `depends:` line names by id (`(id, id)` pairs, the
/// shape `dependent_entries` gives a depends id), leaving out an id already
/// among `already` (the section's other pairs), so a term named both ways
/// earns one entry.
fn depends_term_pairs(
    sec: &record::Section,
    tf: &TermsFile,
    already: &[(String, String)],
) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for d in &sec.depends {
        let record::Dep::Id { id: dep_id, .. } = d else {
            continue;
        };
        if already.iter().any(|(_, u)| u == dep_id) || out.iter().any(|(_, u)| u == dep_id) {
            continue;
        }
        if !tf
            .terms
            .iter()
            .any(|t| t.id.as_deref() == Some(dep_id.as_str()))
        {
            continue;
        }
        out.push((dep_id.clone(), dep_id.clone()));
    }
    out
}

/// The (rel, heading) of every asked question whose `terms:` line is
/// present but cannot be read. `unsettled_entries` cannot tell what such a
/// question names, so a retire it should refuse would pass; the terms
/// approve refuses on these instead. A question with no `terms:` line at
/// all names no term and is not listed.
pub fn unreadable_asked_terms_lines(records: &[RecordFile], led: &Ledger) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for rf in records {
        for sec in &rf.sections {
            let Some(id) = &sec.id else { continue };
            if led.latest_state(id).map(|r| r.event.as_str()) != Some("asked") {
                continue;
            }
            let Some(tl) = sec.terms_line else { continue };
            if terms::parse_terms_line(rf.lines[tl].trim_end()).is_none() {
                out.push((rf.rel.clone(), sec.heading_text.clone()));
            }
        }
    }
    out
}

/// One source's pairing to one or more terms: the pairs themselves, and
/// whether each is a resolved word (subject to "did this word move off the
/// term" checks, since a terms-line entry is a snapshot of what some prose
/// used to resolve to) or a `depends:` id read straight off the entry
/// (never a resolved word, so only the target's removal/retirement applies
/// to it).
pub struct DependentEntry {
    pub source: DependentSource,
    pub pairs: Vec<(String, String)>,
    pub from_word: bool,
}

/// Every current decision's, and (when `include_terms`) every non-retired
/// term's, declared use of a term in `tf` -- the widest reach any of its
/// three readers need: a current decision's own `terms:` line always
/// counts; a `depends:` entry naming a term directly counts too when
/// `include_depends_ids` is set (skipped when the same id is already on
/// that decision's terms line, so retiring a term used both ways earns one
/// entry, not two); a non-retired term's own recorded pairing, from its own
/// `terms_line`, counts when `include_terms` is set. Never transitive on
/// its own -- these pairs are not walked further through the term graph.
pub fn dependent_entries(
    records: &[RecordFile],
    led: &Ledger,
    tf: &TermsFile,
    include_depends_ids: bool,
    include_terms: bool,
) -> Vec<DependentEntry> {
    let mut out = Vec::new();
    for rf in records {
        for sec in &rf.sections {
            let Some(id) = &sec.id else { continue };
            if !is_current(led, id) {
                continue;
            }
            let terms_line_pairs: Vec<(String, String)> = sec
                .terms_line
                .and_then(|tl| terms::parse_terms_line(rf.lines[tl].trim_end()))
                .unwrap_or_default();
            if !terms_line_pairs.is_empty() {
                out.push(DependentEntry {
                    source: DependentSource::Decision {
                        rel: rf.rel.clone(),
                        heading: sec.heading_text.clone(),
                        id: id.clone(),
                    },
                    pairs: terms_line_pairs.clone(),
                    from_word: true,
                });
            }
            if include_depends_ids {
                for d in &sec.depends {
                    let record::Dep::Id { id: dep_id, .. } = d else {
                        continue;
                    };
                    if terms_line_pairs.iter().any(|(_, u)| u == dep_id) {
                        continue;
                    }
                    if !tf
                        .terms
                        .iter()
                        .any(|t| t.id.as_deref() == Some(dep_id.as_str()))
                    {
                        continue;
                    }
                    out.push(DependentEntry {
                        source: DependentSource::Decision {
                            rel: rf.rel.clone(),
                            heading: sec.heading_text.clone(),
                            id: id.clone(),
                        },
                        pairs: vec![(dep_id.clone(), dep_id.clone())],
                        from_word: false,
                    });
                }
            }
        }
    }
    if include_terms {
        for t in tf.terms.iter() {
            if t.retired {
                continue;
            }
            let pairs = t
                .terms_line
                .as_deref()
                .and_then(terms::parse_terms_line)
                .unwrap_or_default();
            if pairs.is_empty() {
                continue;
            }
            out.push(DependentEntry {
                source: DependentSource::Term {
                    name: t.name.clone(),
                },
                pairs,
                from_word: true,
            });
        }
    }
    out
}
