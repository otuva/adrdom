//! The judgements every command reads: one place that runs the rules.
use super::topic::{in_sections, letter_found};
use super::*;
use crate::depends;
use crate::ledger::Ledger;
use crate::msg;
use crate::record::RecordFile;
use crate::repo;
use crate::state;
use crate::status;
use crate::terms;
use std::collections::{HashMap, HashSet};
use std::path::Path;

// --- the judgements every command reads --------------------------------------

/// What one topic file is judged against: the file, every topic file as
/// loaded, the terms, and what the caller built once from them.
pub struct TopicFileJudge<'a> {
    pub root: &'a Path,
    pub rel: &'a str,
    pub rf: &'a RecordFile,
    pub all: &'a [RecordFile],
    pub terms_list: &'a [terms::Term],
    pub phrase_lookup: &'a HashMap<String, String>,
    pub cross_file_headings: &'a HashMap<String, Vec<(String, String)>>,
    pub names: &'a HashMap<String, String>,
    pub replaced: &'a HashSet<String>,
    /// Every blame file the `blame:` lines are judged against, read through
    /// the same `repo::Source` as the topic files of the run.
    pub blame: &'a crate::blame::Index,
    pub led: &'a Ledger,
    /// Whether the planning rules judge the file (`planning_rules_apply`,
    /// read off the file as written).
    pub planning: bool,
    /// What the planning of every file of the run reads, when the caller
    /// judges many files and has read it once; `None` reads it here.
    pub shared: Option<&'a crate::plan::Shared>,
    /// Whether the rules on the directory (the file's own name, a heading
    /// twice in it, a heading in another file) judge it too: `false` for a
    /// caller that judges the whole directory once on its own (`check`).
    pub directory: bool,
}

/// Whether the rules of the planning steps (a question with no body, the
/// reader's rows, every neighbour on a line) judge a topic file: while it
/// awaits approval, its plan on the way to the owner. An approved file is
/// judged by the rules of its record alone, so a heading approved before
/// those rules is no finding. The one scope `approve` and `check` both read
/// (per "Does a command refuse what check passes?", fe14991d A). `rf` is the
/// file as written; `rel` its
/// path from the root, as the ledger names it.
pub fn planning_rules_apply(led: &Ledger, rf: &RecordFile, rel: &str) -> bool {
    status::topic_file_status_at(led, rf, rel).awaits_approval()
}

/// Every rule one topic file breaks on its own, in the order `approve`
/// names them: the rules of the record (`record_refusals`), its own name and
/// headings, a heading in another topic file, an answer-set letter its
/// target has no option for, and, while `planning_rules_apply`, a question
/// not yet asked with no body and the planning records of a question not
/// yet asked (the record of each step and every neighbour on a
/// line). The one judgement `approve` refuses
/// on (per "Does a command refuse what check passes?", fe14991d A).
pub fn judge_topic_file(j: &TopicFileJudge) -> Vec<Finding> {
    let (rel, rf, led) = (j.rel, j.rf, j.led);
    let mut out = about_topic_file(
        record_refusals(
            rel,
            rf,
            j.terms_list,
            j.phrase_lookup,
            j.cross_file_headings,
            j.names,
            j.all,
            j.replaced,
            j.blame,
            led,
        ),
        rel,
    );
    if j.directory {
        out.extend(own_name_and_heading_refusals(
            rel,
            rf.topic.as_deref(),
            &rf.sections,
        ));
        out.extend(cross_file_heading_refusals(rel, rf, j.all, led));
    }
    // Per "Does a depends entry carry the answers it holds under?": a letter
    // the target has no option for is found before `ask`.
    out.extend(about_topic_file(
        in_sections(Class::Plan, letter_found(rel, rf, led)),
        rel,
    ));
    if j.planning {
        out.extend(found(
            Class::Plan,
            rel,
            planning_steps(j.root, rel, rf, j.all, j.terms_list, led, j.shared),
        ));
    }
    out.extend(about_topic_file(
        banned_in_questions(j.root, rel, rf, j.terms_list, led),
        rel,
    ));
    out
}

/// A banned word in the text of a question open to ask (per "What does the
/// ban scan read?" and "Does ban refuse a word that appears in the
/// repository?"): one finding per question, naming the question and never
/// the word. What `ask` refuses a candidate on, read by `show`, `plan` and
/// `approve` alike, so no command names `adr ask` or the owner's approve
/// for a plan holding one. `check` reports the word by file and line in
/// its own scan of every tracked file and does not file these again.
pub fn banned_in_questions(
    root: &Path,
    rel: &str,
    rf: &RecordFile,
    terms_list: &[terms::Term],
    led: &Ledger,
) -> Vec<Finding> {
    let entries: HashMap<String, Option<String>> =
        crate::banned::read_entries(&root.join(crate::banned::BAN_LIST))
            .into_iter()
            .collect();
    if entries.is_empty() {
        return Vec::new();
    }
    rf.sections
        .iter()
        .enumerate()
        .filter(|(_, sec)| status::open_to_ask_text(led, rf, sec))
        .filter_map(|(i, sec)| {
            let text = state::section_text_excluding_terms_line(&rf.lines, sec);
            let hit = crate::banned::first_hit(&text, &entries, terms_list)?;
            let said = msg::banned_word_in_candidate(
                rel,
                &sec.heading_text,
                &msg::term_suffix(hit.term.as_deref()),
            );
            Some(Finding::said(Class::Banned, said).in_section(i))
        })
        .collect()
}

/// The questions asked (or refused) in the topic file `rel` and struck from
/// its plan since: each id whose latest `approved` row names `rel`, whose
/// latest state is `asked` or `refused`, and which no topic file of `all`
/// holds any more, while the plan of `rel` differs from its approval
/// (`stale`, read by the caller off the file as written; `sole_stale` when
/// it is the only such file). One finding per
/// id, whose move is the owner's approve of `rel`, which writes the
/// withdrawn row (per "Does approve withdraw an asked question struck from
/// the plan?"). Not part of `judge_topic_file`: approve clears it rather
/// than refusing on it; show, plan and check print it the same way.
pub fn struck_asked(
    rel: &str,
    stale: bool,
    sole_stale: bool,
    all: &[RecordFile],
    led: &Ledger,
) -> Vec<Finding> {
    if !stale {
        return Vec::new();
    }
    struck_asked_ids(rel, sole_stale, all, led)
        .into_iter()
        .map(|(id, heading, event)| {
            Finding::said(
                Class::Ids,
                msg::removed_from_plan(&id, &heading, event, rel),
            )
            .about(Subject::TopicFile(rel.to_string()))
        })
        .collect()
}

/// The ids `struck_asked` reports for `rel`, each with the heading it was
/// asked under and its latest state, in ledger order: the ids whose latest
/// per-question `approved` row names `rel`, and, when `rel` is the only file
/// awaiting a re-approval (`sole_stale`), the ids asked under a ledger that
/// approved whole files only and so names no file for them.
pub fn struck_asked_ids<'l>(
    rel: &str,
    sole_stale: bool,
    all: &[RecordFile],
    led: &'l Ledger,
) -> Vec<(String, String, &'l str)> {
    let held: HashSet<&str> = held_ids(all);
    let mut seen: HashSet<&str> = HashSet::new();
    let mut out = Vec::new();
    for row in led
        .rows
        .iter()
        .filter(|r| r.event == "asked" && r.id != "-")
    {
        let id = row.id.as_str();
        if !seen.insert(id) || held.contains(id) {
            continue;
        }
        match led.latest_for_id("approved", id).map(|r| r.detail.as_str()) {
            Some(file) if file == rel => {}
            None if sole_stale => {}
            _ => continue,
        }
        let Some(event) = led
            .latest_state(id)
            .map(|r| r.event.as_str())
            .filter(|e| *e == "asked" || *e == "refused")
        else {
            continue;
        };
        let heading = led
            .latest_asked(id)
            .map(|r| r.detail.clone())
            .unwrap_or_else(|| id.to_string());
        out.push((id.to_string(), heading, event));
    }
    out
}

/// The topic files of `all` that were approved once and whose plan differs
/// from that approval now, read off the files as written.
pub fn stale_files<'a>(all: &'a [RecordFile], led: &Ledger) -> Vec<&'a str> {
    all.iter()
        .filter(|rf| {
            led.latest_for_path("approved", &rf.rel).is_some()
                && !state::plan_approved(led, rf, &rf.rel)
        })
        .map(|rf| rf.rel.as_str())
        .collect()
}

/// A question not yet asked with no body, and the planning records of the
/// questions not yet asked: per "What does the agent do to plan a topic
/// file?" and "Does a planning step run without the record of the step
/// before?", the rows are read as they stand; neither the model nor the
/// reader is needed.
fn planning_steps(
    root: &Path,
    rel: &str,
    rf: &RecordFile,
    all: &[RecordFile],
    terms_list: &[terms::Term],
    led: &Ledger,
    shared: Option<&crate::plan::Shared>,
) -> Vec<msg::Said> {
    let mut planning = match shared {
        Some(shared) => crate::plan::load_with(rf, terms_list, led, shared),
        None => crate::plan::load(root, rf, all, terms_list, led),
    };
    planning.rel = rel.to_string();
    // The planning steps run in order, and the findings are those of the
    // step the plan is at, the one `moves::of_planning` names: a later
    // step's records are missing only because this one is not done (per
    // "Does a command compute a move of its own?", aac5c313 A).
    match planning.next() {
        crate::plan::NextMove::Plan => planning.rows_missing_at_step(),
        crate::plan::NextMove::WriteLines => planning.line_refusals(),
        crate::plan::NextMove::WriteBodies(_) => no_body_refusals(rel, rf, led),
        crate::plan::NextMove::Ready => Vec::new(),
    }
}

/// What the judgement of every topic file of a run reads of the whole
/// repository, read once per run rather than once per file: the planning
/// records (`plan::Shared`, with the term phrases and the heading indexes),
/// what the approved plans' supersedes lines replace, the files whose plan
/// differs from an approval and the blame files.
pub struct RepoIndex {
    pub planning: crate::plan::Shared,
    /// Every blame file, the `blame:` lines' side of the judgement.
    pub blame: crate::blame::Index,
    base: HashSet<String>,
    stale: Vec<String>,
    records: Vec<Finding>,
}

impl RepoIndex {
    pub fn new(
        root: &Path,
        all: &[RecordFile],
        terms_list: &[terms::Term],
        led: &Ledger,
    ) -> RepoIndex {
        RepoIndex::new_via(root, all, terms_list, led, &repo::Source::WorkingTree)
    }

    /// `new` with the reader's rows as `source` holds them.
    pub fn new_via(
        root: &Path,
        all: &[RecordFile],
        terms_list: &[terms::Term],
        led: &Ledger,
        source: &repo::Source,
    ) -> RepoIndex {
        let (blame, _) = crate::blame::Index::read(root, source, led);
        RepoIndex::new_with_blame(root, all, terms_list, led, source, blame)
    }

    /// `new_via` with the blame index the caller already holds, read from the
    /// same rows as `source`, instead of reading it again.
    pub fn new_with_blame(
        root: &Path,
        all: &[RecordFile],
        terms_list: &[terms::Term],
        led: &Ledger,
        source: &repo::Source,
        blame: crate::blame::Index,
    ) -> RepoIndex {
        let planning = crate::plan::Shared::new_via(root, all, terms_list, led, source);
        let base = depends::replaced_base(
            all,
            &depends::approved_plan_rels(all, led),
            &planning.cross,
            led,
        );
        let stale = stale_files(all, led)
            .into_iter()
            .map(str::to_string)
            .collect();
        let records = judge_records(all, led);
        RepoIndex {
            planning,
            blame,
            base,
            stale,
            records,
        }
    }

    /// The rules on the topic files as a whole (`judge_records`).
    pub fn records(&self) -> &[Finding] {
        &self.records
    }

    /// The files whose plan differs from an approval (`stale_files`).
    pub fn stale(&self) -> &[String] {
        &self.stale
    }
}

/// `judge_topic_file` of the file as loaded, judged the way approve judges
/// it, with what the run shares read once (`index`).
pub fn judge_topic_file_in(
    root: &Path,
    rf: &RecordFile,
    all: &[RecordFile],
    terms_list: &[terms::Term],
    led: &Ledger,
    index: &RepoIndex,
) -> Vec<Finding> {
    let shared = &index.planning;
    let replaced = depends::replaced_ids(&index.base, rf, &shared.cross, led);
    judge_topic_file(&TopicFileJudge {
        root,
        rel: &rf.rel,
        rf,
        all,
        terms_list,
        phrase_lookup: &shared.phrase_lookup,
        cross_file_headings: &shared.cross,
        names: &shared.headings,
        replaced: &replaced,
        blame: &index.blame,
        led,
        planning: planning_rules_apply(led, rf, &rf.rel),
        shared: Some(shared),
        directory: true,
    })
}

/// Per "Does the agent write the bodies before approve?": the plan is
/// approved with its bodies, so a question never asked that has no body is
/// a finding.
pub fn no_body_refusals(rel: &str, rf: &RecordFile, led: &Ledger) -> Vec<msg::Said> {
    rf.sections
        .iter()
        .filter(|sec| sec.is_skeleton)
        .filter(|sec| {
            !sec.id
                .as_ref()
                .is_some_and(|id| led.latest_asked(id).is_some())
        })
        .map(|sec| msg::question_without_body(rel, &sec.heading_text))
        .collect()
}

/// The rules on the records as a whole: a current decision whose id no
/// topic file holds (`missing_answered_refusals`).
pub fn judge_records(records: &[RecordFile], led: &Ledger) -> Vec<Finding> {
    judge_records_except(records, led, &HashSet::new())
}

/// The terms a topic file is judged against, by every command that judges
/// one: the terms file with an identifier on every entry
/// (`terms::minted_copy`), so a word an entry still waiting for its
/// approval defines pairs to it, and the finding is the terms file's own
/// approval (`terms_gate`), never a missing term the file already holds.
pub fn judged_terms(tf: Option<&terms::TermsFile>) -> Vec<terms::Term> {
    tf.map(|tf| terms::minted_copy(tf).terms)
        .unwrap_or_default()
}

/// What keeps `approve`, `ask` and `answer` from taking the topic file
/// `rel` while adrdom/terms.md is missing, unreadable, malformed or waits
/// for its approval (per "Does the tool stop while the terms file waits for
/// approval?"): the gate `actions::require_terms` stops them on, as
/// findings of the file, so `show` and `plan` name the move that clears it
/// and no move those commands would refuse.
pub fn terms_gate(root: &Path, led: &Ledger, rel: &str) -> TermsGate {
    let mut gate = TermsGate {
        waits: crate::terms::load(&root.join(crate::repo::TERMS_PATH))
            .is_ok_and(|tf| !state::terms_approved(led, &tf)),
        ..TermsGate::default()
    };
    if let Err(lines) = crate::actions::require_terms(root, led, rel, Command::Show) {
        for l in lines {
            match l {
                crate::out::Line::Finding(f) => gate.findings.push(f.about(Subject::TermsFile)),
                // What stopped the gate on the terms file's own input or
                // files stays the stop line it is.
                stop @ crate::out::Line::Stop { .. } => gate.stops.push(stop),
                _ => {}
            }
        }
    }
    gate
}

/// What `terms_gate` found: the findings of the terms file and the stops on
/// its own input or files.
#[derive(Debug, Default)]
pub struct TermsGate {
    pub findings: Vec<Finding>,
    pub stops: Vec<crate::out::Line>,
    /// The terms file is there and differs from its latest approval: a
    /// commit waits for the owner's approve.
    pub waits: bool,
}

impl TermsGate {
    pub fn is_empty(&self) -> bool {
        self.findings.is_empty() && self.stops.is_empty()
    }
}

/// `judge_records` leaving out of the missing-answered rule the ids in
/// `skip` (`missing_answered_refusals_except`): every rule on the topic
/// files as a whole, the one judgement `check` fails, `approve` refuses and
/// `show` and `plan` print with a file's report. An identifier held by two
/// questions, a withdrawn identifier back in a file (a withdrawn identifier
/// never returns, per "Does approve withdraw an asked question struck from
/// the plan?"), and a current decision no topic file holds.
pub fn judge_records_except(
    records: &[RecordFile],
    led: &Ledger,
    skip: &HashSet<String>,
) -> Vec<Finding> {
    let mut out = Vec::new();
    let mut seen: HashMap<&str, String> = HashMap::new();
    for rf in records {
        for sec in rf.sections.iter().filter(|s| !s.is_skeleton) {
            let Some(id) = sec.id.as_deref() else {
                continue;
            };
            match seen.get(id) {
                Some(prev) => out.push(
                    Finding::said(
                        Class::Ids,
                        msg::duplicate_id(id, prev, &rf.rel, &sec.heading_text),
                    )
                    .about(Subject::TopicFile(rf.rel.clone())),
                ),
                None => {
                    seen.insert(id, format!("{}:{}", rf.rel, sec.heading_text));
                }
            }
        }
    }
    for rf in records {
        for sec in &rf.sections {
            let Some(id) = &sec.id else { continue };
            if led
                .latest_state(id)
                .is_some_and(|row| row.event == "withdrawn")
            {
                out.push(
                    Finding::said(
                        Class::Ids,
                        msg::withdrawn_id_present(id, &sec.heading_text, &rf.rel),
                    )
                    .about(Subject::TopicFile(rf.rel.clone())),
                );
            }
        }
    }
    out.extend(missing_answered_refusals_except(records, led, skip));
    out
}

/// The rules on a removed topic file: each question it held that keeps its
/// removal from being approved (`removal_holds`).
pub fn judge_removed_topic_file(
    root: &Path,
    rel: &str,
    led: &Ledger,
    present_ids: &HashSet<String>,
) -> Vec<Finding> {
    removal_holds(root, rel, led, present_ids)
        .holds
        .iter()
        .map(|h| {
            Finding::said(Class::Plan, msg::removal_holds_open(rel, h))
                .about(Subject::TopicFile(rel.to_string()))
        })
        .collect()
}

/// What approve would refuse in the terms file `tf` (minted) as it stands
/// in the working tree, banned words in its text included: the one terms
/// judgement `show` prints and the gate of `ask`, `answer` and the approve
/// of a topic file names, so no command offers the owner's approve of the
/// terms file while approve would refuse it.
pub fn terms_file_findings(
    root: &Path,
    records: &[RecordFile],
    tf: &terms::TermsFile,
    led: &Ledger,
) -> Vec<Finding> {
    let approved_plans: HashSet<String> = records
        .iter()
        .filter(|rf| state::plan_approved(led, rf, &rf.rel))
        .map(|rf| rf.rel.clone())
        .collect();
    judge_terms_file(
        root,
        records,
        tf,
        led,
        &repo::Source::WorkingTree,
        true,
        &approved_plans,
    )
    .0
}

/// The rules on the terms file (`terms_refusals`), each a finding about it;
/// the second part is what `terms_refusals` returns beside the messages (the
/// pairings `approve` rewrites).
#[allow(clippy::type_complexity)]
pub fn judge_terms_file(
    root: &Path,
    records: &[RecordFile],
    tf: &terms::TermsFile,
    led: &Ledger,
    source: &repo::Source,
    scan_banned: bool,
    approved_plans: &HashSet<String>,
) -> (Vec<Finding>, Vec<Vec<(String, String)>>) {
    terms_refusals(root, records, tf, led, source, scan_banned, approved_plans)
}

/// The coverage rule of a blame file over an origin of `n` lines (per "Does a
/// blame file cover its whole origin?"): each range past the origin's end and
/// each run of lines no row covers.
pub fn judge_blame_coverage(bf: &crate::blame::BlameFile, n: usize) -> Vec<Finding> {
    let (beyond, gaps) = crate::blame::coverage_errors(&bf.rows, n);
    let about = || Subject::BlameFile(bf.rel.clone());
    let mut out: Vec<Finding> = beyond
        .iter()
        .map(|b| {
            Finding::said(
                Class::Blame,
                msg::blame_range_beyond(&bf.rel, b.start, b.end, n),
            )
            .about(about())
        })
        .collect();
    out.extend(gaps.iter().map(|g| {
        Finding::said(
            Class::Blame,
            msg::blame_coverage_gap(&bf.rel, g.start, g.end),
        )
        .about(about())
    }));
    out
}

/// Every rule one blame file breaks, read off its state against its origin
/// (`blame::file_state`): the findings of the state, then, when the origin
/// could be read, coverage. What `approve` refuses on, `check` prints and
/// `show` names the moves of a blame file from.
pub fn judge_blame_file(
    st: &crate::blame::FileState,
    bf: &crate::blame::BlameFile,
) -> Vec<Finding> {
    let mut out = st.findings.clone();
    if let Some(c) = &st.origin_content {
        out.extend(judge_blame_coverage(bf, c.lines().count()));
    }
    out
}
