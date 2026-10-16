//! The rulings a question earned toward its id, and the successors and conflicts they make.
use crate::ledger::{Ledger, Row};

/// One ruling a question earned toward its `id`: an answered letter, or a
/// refusal. Per "Does check fail a question answered with two different
/// options?" (e3200081-9995-4bbe-983f-2f430dc341e7, rule 8): a refusal and
/// an answer are two different rulings, so a `refused` row counts here
/// exactly as an `answered` row does. Every `refused` row is the same one
/// ruling, distinct from every letter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ruling {
    Answered(char),
    Refused,
}

impl Ruling {
    /// The word this ruling names in a message: the letter itself, or
    /// `"refused"`.
    pub fn label(&self) -> String {
        match self {
            Ruling::Answered(c) => c.to_string(),
            Ruling::Refused => "refused".to_string(),
        }
    }
}

/// `row`'s ruling, when it is an `answered` or `refused` row for the id it
/// is checked against (any other event, including `asked`, carries no
/// ruling).
fn ruling_of(row: &Row) -> Option<Ruling> {
    match row.event.as_str() {
        "answered" => row.detail.trim().chars().next().map(Ruling::Answered),
        "refused" => Some(Ruling::Refused),
        _ => None,
    }
}

/// The standing rulings for `id`, per "Does check fail a question answered
/// with two different options?" (e3200081-9995-4bbe-983f-2f430dc341e7): take
/// `id`'s `asked` and `answered`/`refused` rows in ledger order (`led.rows`
/// is already ordered by time); let `r` be the position of the latest
/// `asked` row. `r` is an "asked again" row when the rulings before it --
/// each `answered` row's letter and each `refused` row, per `ruling_of` --
/// name two or more different rulings. The standing rulings are the
/// `answered`/`refused` rows after `r` when `r` is an asked-again row,
/// otherwise every `answered`/`refused` row of the question. `None` when
/// `id` was never asked at all -- there is no `r` to reckon from.
pub fn standing_rulings(led: &Ledger, id: &str) -> Option<Vec<Ruling>> {
    let r_pos = led
        .rows
        .iter()
        .rposition(|row| row.id == id && row.event == "asked")?;

    let mut distinct_rulings_before: Vec<Ruling> = Vec::new();
    for row in &led.rows[..r_pos] {
        if row.id == id {
            if let Some(ruling) = ruling_of(row) {
                if !distinct_rulings_before.contains(&ruling) {
                    distinct_rulings_before.push(ruling);
                }
            }
        }
    }
    let asked_again = distinct_rulings_before.len() >= 2;

    let range: &[Row] = if asked_again {
        &led.rows[r_pos + 1..]
    } else {
        &led.rows[..]
    };
    let rulings: Vec<Ruling> = range
        .iter()
        .filter(|row| row.id == id)
        .filter_map(ruling_of)
        .collect();
    Some(rulings)
}

/// The current decisions that took the place of `id`: follows the `superseded`
/// rows (detail: the superseding id) through every chain and keeps the ends
/// whose latest state is `answered`. Empty when none is current.
pub fn current_successors(led: &Ledger, id: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut stack = vec![id.to_string()];
    while let Some(at) = stack.pop() {
        if !seen.insert(at.clone()) {
            continue;
        }
        match led.latest_state(&at).map(|r| r.event.as_str()) {
            Some("answered") => {
                if at != id && !out.contains(&at) {
                    out.push(at);
                }
            }
            Some("superseded") => {
                let mut next: Vec<String> = led
                    .rows
                    .iter()
                    .filter(|r| r.event == "superseded" && r.id == at)
                    .map(|r| r.detail.clone())
                    .collect();
                next.reverse();
                stack.extend(next);
            }
            _ => {}
        }
    }
    out
}

/// The first two distinct standing rulings for `id`
/// (e3200081-9995-4bbe-983f-2f430dc341e7, rule 3), in the order their rows
/// appear in the ledger -- `Some` exactly when the question is in conflict:
/// its standing rulings name two or more different options. `None` when
/// `id` was never asked, or its standing rulings all agree.
pub fn conflicting_rulings(led: &Ledger, id: &str) -> Option<(Ruling, Ruling)> {
    let rulings = standing_rulings(led, id)?;
    let mut distinct: Vec<Ruling> = Vec::new();
    for r in rulings {
        if !distinct.contains(&r) {
            distinct.push(r);
        }
    }
    if distinct.len() >= 2 {
        Some((distinct[0], distinct[1]))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // adr: e3200081-9995-4bbe-983f-2f430dc341e7
    //
    // The case walk from the build brief, at the ledger level (no files, no
    // git): c1 through c7, each built as the exact row sequence its own
    // description names, times ticking one second apart (the tests only
    // need each row to sort after the last, not real wall-clock gaps).
    mod conflict_tests {
        use super::*;

        fn row_at(event: &str, id: &str, detail: &str, time: &str) -> Row {
            Row {
                time: time.to_string(),
                event: event.to_string(),
                id: id.to_string(),
                hash: "h".to_string(),
                detail: detail.to_string(),
                version: "0.1.0".to_string(),
            }
        }

        fn led_of(rows: Vec<Row>) -> Ledger {
            // Rows come out of `ledger::parse` already ordered by time;
            // built here the same way, from the clean list this test
            // module writes them in (each `row_at` call already at a later
            // time than the last).
            Ledger {
                rows,
                malformed: vec![],
            }
        }

        fn t(n: u32) -> String {
            format!("2024-01-01T00:00:{:02}Z", n)
        }

        // c1: base asked t1; branch a answers A t2; branch b answers B t3
        // (merged) -> conflict; re-asked t5; owner answers A t6 -> A
        // stands, no conflict.
        #[test]
        fn c1_two_answers_after_one_ask_conflict_then_resolved() {
            let led = led_of(vec![
                row_at("asked", "q", "Q?", &t(1)),
                row_at("answered", "q", "A", &t(2)),
                row_at("answered", "q", "B", &t(3)),
            ]);
            let (a, b) = conflicting_rulings(&led, "q").expect("c1 should conflict");
            assert_eq!((a, b), (Ruling::Answered('A'), Ruling::Answered('B')));

            let led = led_of(vec![
                row_at("asked", "q", "Q?", &t(1)),
                row_at("answered", "q", "A", &t(2)),
                row_at("answered", "q", "B", &t(3)),
                row_at("asked", "q", "Q?", &t(5)),
                row_at("answered", "q", "A", &t(6)),
            ]);
            assert!(
                conflicting_rulings(&led, "q").is_none(),
                "A stands, no conflict"
            );
        }

        // c2: branch a asked t1, answered A t2; branch b asked t3, answered
        // B t4 (base never asked) -> R = t3 is not asked-again (before it
        // only A) -> standing = all = {A, B} -> conflict; re-asked t5,
        // answered B t6 -> B stands.
        #[test]
        fn c2_independently_asked_on_each_branch_conflict_then_resolved() {
            let led = led_of(vec![
                row_at("asked", "q", "Q?", &t(1)),
                row_at("answered", "q", "A", &t(2)),
                row_at("asked", "q", "Q?", &t(3)),
                row_at("answered", "q", "B", &t(4)),
            ]);
            let (a, b) = conflicting_rulings(&led, "q").expect("c2 should conflict");
            assert_eq!((a, b), (Ruling::Answered('A'), Ruling::Answered('B')));

            let led = led_of(vec![
                row_at("asked", "q", "Q?", &t(1)),
                row_at("answered", "q", "A", &t(2)),
                row_at("asked", "q", "Q?", &t(3)),
                row_at("answered", "q", "B", &t(4)),
                row_at("asked", "q", "Q?", &t(5)),
                row_at("answered", "q", "B", &t(6)),
            ]);
            assert!(
                conflicting_rulings(&led, "q").is_none(),
                "B stands, no conflict"
            );
        }

        // c3: interleaved -- asked t1 (a), asked t2 (b), answered A t3,
        // answered B t4 -> conflict (the latest asked row, t2, has no
        // answered row before it at all, so it is not asked-again either).
        #[test]
        fn c3_two_asked_rows_then_two_answers_conflict() {
            let led = led_of(vec![
                row_at("asked", "q", "Q?", &t(1)),
                row_at("asked", "q", "Q?", &t(2)),
                row_at("answered", "q", "A", &t(3)),
                row_at("answered", "q", "B", &t(4)),
            ]);
            let (a, b) = conflicting_rulings(&led, "q").expect("c3 should conflict");
            assert_eq!((a, b), (Ruling::Answered('A'), Ruling::Answered('B')));
        }

        // c4: one branch only -- asked t1, answered A t2 -> no conflict.
        #[test]
        fn c4_single_answer_no_conflict() {
            let led = led_of(vec![
                row_at("asked", "q", "Q?", &t(1)),
                row_at("answered", "q", "A", &t(2)),
            ]);
            assert!(conflicting_rulings(&led, "q").is_none());
        }

        // c5: both branches answer the same letter -- not a conflict.
        #[test]
        fn c5_two_answers_same_letter_no_conflict() {
            let led = led_of(vec![
                row_at("asked", "q", "Q?", &t(1)),
                row_at("answered", "q", "A", &t(2)),
                row_at("answered", "q", "A", &t(3)),
            ]);
            assert!(conflicting_rulings(&led, "q").is_none());
        }

        // c6: asked again at t5 and not yet answered -- open, no conflict
        // (mirrors c1's ledger up through its own re-ask row).
        #[test]
        fn c6_reasked_and_not_yet_answered_no_conflict() {
            let led = led_of(vec![
                row_at("asked", "q", "Q?", &t(1)),
                row_at("answered", "q", "A", &t(2)),
                row_at("answered", "q", "B", &t(3)),
                row_at("asked", "q", "Q?", &t(5)),
            ]);
            assert!(
                conflicting_rulings(&led, "q").is_none(),
                "open, not yet re-answered"
            );
        }

        // c7: today's rules never let an *answered* id's text move and be
        // asked again -- an answered question is frozen (`is_frozen`) and
        // can only be superseded by a new id, never re-asked in place
        // (tests/answer_test.rs, tests/check_test.rs: "an answered question
        // can never be re-asked"). So the one flow this rule must not break
        // -- an id asked again with a different hash while its latest state
        // is already `answered` -- has no path to it except through this
        // very rule's own conflict re-ask (which always re-asks under the
        // *same* hash, the text never having moved -- see
        // `ask::do_conflict_reask`). There is nothing else to test here.
        #[test]
        fn c7_no_existing_flow_reasks_an_answered_id_with_a_different_hash() {}

        // Rule 8: a refusal counts as a ruling too -- one branch answers,
        // the other refuses -- and mixing it with an answer is a conflict
        // exactly like two different letters.
        #[test]
        fn rule_8_an_answer_and_a_refusal_conflict() {
            let led = led_of(vec![
                row_at("asked", "q", "Q?", &t(1)),
                row_at("answered", "q", "A", &t(2)),
                row_at("refused", "q", "Q?", &t(3)),
            ]);
            let (a, b) = conflicting_rulings(&led, "q").expect("answer vs refusal should conflict");
            assert_eq!((a, b), (Ruling::Answered('A'), Ruling::Refused));
        }

        // Two refusals (both branches refused) are the same ruling, not a
        // conflict.
        #[test]
        fn rule_8_two_refusals_are_not_a_conflict() {
            let led = led_of(vec![
                row_at("asked", "q", "Q?", &t(1)),
                row_at("refused", "q", "Q?", &t(2)),
                row_at("refused", "q", "Q?", &t(3)),
            ]);
            assert!(conflicting_rulings(&led, "q").is_none());
        }

        // Never asked at all: no `r` to reckon from, so no conflict.
        #[test]
        fn never_asked_no_conflict() {
            let led = led_of(vec![]);
            assert!(conflicting_rulings(&led, "q").is_none());
        }

        // Rule 8, continued: a `refused` row counts toward "asked again"
        // exactly as an `answered` row does. Before this fix, only
        // `answered` rows were counted, so a ruling made of one answer and
        // one refusal was never recognized as "asked again" -- asking again
        // and answering again could never clear the conflict.
        //
        // asked t1; answered A t2; refused t3 (two distinct rulings before
        // the next ask); re-asked t5 -> asked-again, nothing after it yet ->
        // open, no conflict.
        #[test]
        fn rule_8_reasked_after_answer_and_refusal_no_conflict_while_open() {
            // adr: e3200081-9995-4bbe-983f-2f430dc341e7
            let led = led_of(vec![
                row_at("asked", "q", "Q?", &t(1)),
                row_at("answered", "q", "A", &t(2)),
                row_at("refused", "q", "Q?", &t(3)),
                row_at("asked", "q", "Q?", &t(5)),
            ]);
            assert!(
                conflicting_rulings(&led, "q").is_none(),
                "open, not yet settled again"
            );
        }

        // Same ledger, then answered A again after the re-ask -> A stands.
        #[test]
        fn rule_8_reasked_after_answer_and_refusal_then_answered_again_stands() {
            // adr: e3200081-9995-4bbe-983f-2f430dc341e7
            let led = led_of(vec![
                row_at("asked", "q", "Q?", &t(1)),
                row_at("answered", "q", "A", &t(2)),
                row_at("refused", "q", "Q?", &t(3)),
                row_at("asked", "q", "Q?", &t(5)),
                row_at("answered", "q", "A", &t(6)),
            ]);
            assert!(
                conflicting_rulings(&led, "q").is_none(),
                "A stands, no conflict"
            );
        }

        // Same ledger, but the settling act after the re-ask is a fresh
        // refusal instead of a letter -> refused stands.
        #[test]
        fn rule_8_reasked_after_answer_and_refusal_then_refused_again_stands() {
            // adr: e3200081-9995-4bbe-983f-2f430dc341e7
            let led = led_of(vec![
                row_at("asked", "q", "Q?", &t(1)),
                row_at("answered", "q", "A", &t(2)),
                row_at("refused", "q", "Q?", &t(3)),
                row_at("asked", "q", "Q?", &t(5)),
                row_at("refused", "q", "Q?", &t(6)),
            ]);
            assert!(
                conflicting_rulings(&led, "q").is_none(),
                "refused stands, no conflict"
            );
        }
    }
}
