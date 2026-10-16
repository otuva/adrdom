//! The approval state of files: which questions, topic files, terms files and blame files the ledger
//! says are dropped, superseded or approved.
use super::hash_matches_latest;
use crate::ledger::{Ledger, Row};
use crate::record::{self, RecordFile};
use crate::terms::{self, TermsFile};

/// True when a `dropped` row names `id`: the question left its topic file
/// when an answer's set lacked the letter (per "Does answer remove a
/// question whose answer set lacks the answer?"). A reference on another
/// question's line to such an id is a known, dropped question.
pub fn is_dropped(led: &Ledger, id: &str) -> bool {
    led.rows.iter().any(|r| r.event == "dropped" && r.id == id)
}

/// The hashes of the questions dropped from the file `rel` since its
/// latest approval: every `dropped` row after the latest file `approved`
/// row whose id and hash match a per-question `approved` row for `rel`
/// (written by `answer`, per "What does answer write for a removed
/// question?"). A drop before that approval is already out of the hash it
/// wrote. Rows are in ledger order, so position says which came later.
pub fn dropped_hashes(led: &Ledger, rel: &str) -> Vec<String> {
    let since = led
        .rows
        .iter()
        .rposition(|r| r.event == "approved" && r.detail == rel && r.id == "-")
        .map_or(0, |i| i + 1);
    led.rows[since..]
        .iter()
        .filter(|d| d.event == "dropped")
        .filter(|d| {
            led.rows.iter().any(|a| {
                a.event == "approved" && a.id == d.id && a.hash == d.hash && a.detail == rel
            })
        })
        .map(|d| d.hash.clone())
        .collect()
}

/// True when the derived hash of `question_hashes` (a file's current
/// questions, computed by the caller) matches the latest `approved` file
/// row for `rel`, either as it stands or with the questions dropped since
/// approval put back (per "Does approve hash each question on its own?": a
/// removed question takes its hash with it and nothing else moves). See
/// `hash_matches_latest`.
pub fn plan_approved_hashes(led: &Ledger, rel: &str, question_hashes: &[String]) -> bool {
    if hash_matches_latest(
        led,
        "approved",
        rel,
        &record::derive_file_hash(question_hashes),
    ) {
        return true;
    }
    let mut base: Vec<String> = question_hashes.to_vec();
    let dropped = dropped_hashes(led, rel);
    let has_dropped = !dropped.is_empty();
    base.extend(dropped);
    if has_dropped && hash_matches_latest(led, "approved", rel, &record::derive_file_hash(&base)) {
        return true;
    }
    // A superseded section struck from the file (per "What strikes the
    // section of a superseded question from its topic file?") takes its
    // hash with it and nothing else moves. A candidate is a superseded
    // question whose latest per-question `approved` row names this file and
    // which the file does not hold. One superseded after the file's latest
    // approval was in the file at that approval, so it is put back; one
    // superseded before it was absent from a file approved since the strike
    // and is put back only for a file approved while it still held it.
    // adr-code: b5ca161f-c88c-4f02-a9d9-1d406cf3a252
    let file_row = led
        .rows
        .iter()
        .rposition(|r| r.event == "approved" && r.detail == rel && r.id == "-");
    let Some(file_row) = file_row else {
        return false;
    };
    let latest = &led.rows[file_row].hash;
    let (mut after, mut before): (Vec<String>, Vec<String>) = (Vec::new(), Vec::new());
    // One pass: the latest state event of each id, the latest per-question
    // `approved` row naming `rel`, and the latest `superseded` position.
    let mut state: std::collections::BTreeMap<&str, &str> = Default::default();
    let mut approved: std::collections::BTreeMap<&str, &Row> = Default::default();
    let mut sup_at: std::collections::BTreeMap<&str, usize> = Default::default();
    for (i, r) in led.rows.iter().enumerate() {
        if crate::ledger::STATE_EVENTS.contains(&r.event.as_str()) {
            state.insert(&r.id, &r.event);
            if r.event == "superseded" {
                sup_at.insert(&r.id, i);
            }
        }
        if r.event == "approved" && r.detail == rel {
            approved.insert(&r.id, r);
        }
    }
    for (id, ev) in &state {
        if *ev != "superseded" {
            continue;
        }
        let Some(r) = approved.get(id) else { continue };
        // A hash holds its question's id line, so no two candidates share one.
        if base.contains(&r.hash) {
            continue;
        }
        let at = sup_at.get(id).copied().unwrap_or(0);
        if at > file_row {
            after.push(r.hash.clone());
        } else {
            before.push(r.hash.clone());
        }
    }
    base.extend(after);
    if !before.is_empty() && record::derive_file_hash(&base) == *latest {
        return true;
    }
    base.extend(before);
    record::derive_file_hash(&base) == *latest
}

/// Every id whose latest state is `superseded`, sorted: the questions whose
/// section no topic file keeps (per "What strikes the section of a
/// superseded question from its topic file?").
pub fn superseded_ids(led: &Ledger) -> Vec<String> {
    led.state_ids()
        .into_iter()
        .filter(|id| is_superseded(led, id))
        .collect()
}

/// True when the latest state of `id` is `superseded`.
pub fn is_superseded(led: &Ledger, id: &str) -> bool {
    led.latest_state(id)
        .is_some_and(|r| r.event == "superseded")
}

/// True when `id` is a superseded question whose latest per-question
/// `approved` row names `rel`: its section was in that topic file and is
/// struck from it (per "What strikes the section of a superseded question
/// from its topic file?").
pub fn is_struck_from(led: &Ledger, id: &str, rel: &str) -> bool {
    is_superseded(led, id)
        && led
            .latest_for_id("approved", id)
            .is_some_and(|r| r.detail == rel)
}

/// True when `context_hash` (computed by the caller) matches the latest
/// `context` ledger row for `rel`. See `hash_matches_latest`.
pub fn context_approved_hash(led: &Ledger, rel: &str, context_hash: &str) -> bool {
    hash_matches_latest(led, "context", rel, context_hash)
}

/// True when the topic file's current questions match the latest `approved`
/// ledger row for `rel` (the topic file's path relative to the repo root).
/// `rf` is the file as written.
pub fn plan_approved(led: &Ledger, rf: &RecordFile, rel: &str) -> bool {
    plan_approved_hashes(led, rel, &record::question_hashes(rf))
}

/// True when the topic file's current context hash matches the latest
/// `context` ledger row for `rel`. Context hashing never reads a
/// section's `depends:` entries; kept alongside `plan_approved` (and
/// `context_approved_hash` given the same way).
pub fn context_approved(led: &Ledger, rf: &RecordFile, rel: &str) -> bool {
    context_approved_hash(led, rel, &record::context_hash(rf))
}

/// True when the terms file's current hash matches the latest `terms`
/// ledger row for `adrdom/terms.md`.
pub fn terms_approved(led: &Ledger, tf: &TermsFile) -> bool {
    match led.latest_for_path("terms", crate::repo::TERMS_PATH) {
        Some(row) => row.hash == terms::file_hash(tf),
        None => false,
    }
}

/// True when `content` (a blame file's current text, exactly as it stands)
/// hashes to the latest `blame` ledger row for `rel` (per "Does the owner
/// approve a blame file?", e879fe72 A: "a blame file that differs from its
/// latest approval is a draft"). Hashed the same way `crate::blame::file_hash`
/// hashes the lines, id line included, `do_approve_blame` is about to write,
/// so a file freshly approved always reads back as approved.
pub fn blame_approved(led: &Ledger, rel: &str, content: &str) -> bool {
    let lines = record::split_lines(content);
    let hash = crate::blame::file_hash(&lines);
    match led.latest_blame(rel) {
        Some(row) => row.hash == hash,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // adr: e879fe72-ec07-48cd-9902-2b158aaae726
    //
    // "a blame file that differs from its latest approval is a draft":
    // no `blame` row at all, or one whose hash doesn't match the content's
    // own hash, is not approved; a row over the exact hash `blame::file_hash`
    // computes for that content is.
    #[test]
    fn blame_approved_compares_the_latest_blame_row_hash() {
        let content =
            "origin: docs/x.md\nblob: 0000000000000000000000000000000000000000\n\n1-1\tdetail\n";
        let led = Ledger {
            rows: vec![],
            malformed: vec![],
        };
        assert!(!blame_approved(&led, "adrdom/blame/x.md", content));

        let hash = crate::blame::file_hash(&record::split_lines(content));
        let led = Ledger {
            rows: vec![Row {
                time: "2024-01-01T00:00:00Z".to_string(),
                event: "blame".to_string(),
                id: "11111111-1111-4111-8111-111111111111".to_string(),
                hash,
                detail: "adrdom/blame/x.md".to_string(),
                version: "0.1.0".to_string(),
            }],
            malformed: vec![],
        };
        assert!(blame_approved(&led, "adrdom/blame/x.md", content));
        assert!(!blame_approved(&led, "adrdom/blame/other.md", content));
        assert!(!blame_approved(
            &led,
            "adrdom/blame/x.md",
            "different content\n"
        ));
    }
}
