//! Rules on a topic file removed from the working tree.
use super::*;
use crate::ledger::Ledger;
use crate::moves;
use crate::record::{self, RecordFile};
use crate::state;
use crate::status;
use std::collections::{BTreeSet, HashSet};
use std::path::Path;

/// Every topic file the ledger holds an approved plan for that is absent
/// from the content judged (`exists`) and whose latest approved plan is not
/// the empty one: a file removed, its removal awaiting the owner's approval
/// (per "What may leave a topic file by a plan change?").
pub fn removal_awaiting(led: &Ledger, exists: &dyn Fn(&str) -> bool) -> Vec<String> {
    let rels: BTreeSet<&str> = led
        .rows
        .iter()
        .filter(|r| r.event == "approved" && r.id == "-" && is_topic_file_path(&r.detail))
        .map(|r| r.detail.as_str())
        .collect();
    let empty = empty_plan_hash();
    rels.into_iter()
        .filter(|rel| {
            led.latest_for_path("approved", rel)
                .is_some_and(|r| r.hash != empty)
        })
        .filter(|rel| !exists(rel))
        .map(|rel| rel.to_string())
        .collect()
}

/// What a removed topic file still holds that no live file does.
pub struct RemovalHolds {
    /// Every id the file held with its heading, in file order.
    pub held: Vec<(String, String)>,
    /// What keeps the removal from being approved: a current decision, an
    /// asked question, a heading of the approved plan never asked.
    pub holds: Vec<moves::RemovalHold>,
}

/// The committed version of `rel` that holds the file's latest approved
/// plan: walks the commits that touched the path newest first, skipping
/// those where it is absent (a deletion), and stops at the first version
/// whose plan matches the latest file-level `approved` row. When no
/// committed version matches (the latest approval was never committed), falls
/// back to the newest committed version that has the file. That fallback can
/// name a heading already struck by an uncommitted approval; the move stays
/// executable: restore the file, strike the heading again, approve.
pub(crate) fn latest_approved_version(
    root: &Path,
    rel: &str,
    led: &Ledger,
) -> Option<std::sync::Arc<RecordFile>> {
    // An approved removal is the empty plan: no version to read.
    if led
        .latest_for_path("approved", rel)
        .is_some_and(|r| r.hash == empty_plan_hash())
    {
        return None;
    }
    let mut newest: Option<std::sync::Arc<RecordFile>> = None;
    for old in crate::approval::committed_topics(root, rel) {
        if state::plan_approved_hashes(led, rel, &record::question_hashes(&old)) {
            return Some(old);
        }
        if newest.is_none() {
            newest = Some(old);
        }
    }
    newest
}

/// What the topic file `rel`, gone from the content judged, held: the ids
/// of its latest approved committed plan (see `latest_approved_version`) and the ids whose latest approved row
/// names it and that have a ledger state, minus the ids in `present_ids`. Shared by `approve`, `check`
/// and `show`, so the three agree on whether a removal passes.
pub fn removal_holds(
    root: &Path,
    rel: &str,
    led: &Ledger,
    present_ids: &HashSet<String>,
) -> RemovalHolds {
    let mut held: Vec<(String, String)> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    if let Some(old) = latest_approved_version(root, rel, led) {
        for sec in &old.sections {
            if let Some(id) = &sec.id {
                if seen.insert(id.clone()) {
                    held.push((id.clone(), sec.heading_text.clone()));
                }
            }
        }
    }
    for r in led
        .rows
        .iter()
        .filter(|r| r.event == "approved" && r.id != "-" && r.detail == rel)
    {
        // approve writes a per-question row only when the question's hash
        // changed, so a heading approved, never asked, then struck keeps a
        // row the ledger cannot tell from a live one. Only an id with a
        // ledger state counts from the ledger; an unasked heading counts
        // from the file's latest approved committed plan alone.
        if led.latest_state(&r.id).is_some()
            && seen.insert(r.id.clone())
            && led
                .latest_for_id("approved", &r.id)
                .is_some_and(|l| l.detail == rel)
        {
            let heading = led
                .latest_asked(&r.id)
                .map(|a| a.detail.clone())
                .unwrap_or_else(|| r.id.clone());
            held.push((r.id.clone(), heading));
        }
    }
    held.retain(|(id, _)| !present_ids.contains(id));

    let mut holds = Vec::new();
    for (id, heading) in &held {
        // The status the closed list names (`status::question_status_of_id`):
        // a question never asked, asked, or a current decision holds the
        // removal; a closed one does not.
        let status = match status::question_status_of_id(led, id) {
            None => status::QuestionStatus::NotAsked,
            Some(st @ (status::QuestionStatus::Asked | status::QuestionStatus::Answered)) => st,
            Some(_) => continue,
        };
        holds.push(moves::RemovalHold {
            id: id.clone(),
            heading: heading.clone(),
            status,
        });
    }
    RemovalHolds { held, holds }
}
