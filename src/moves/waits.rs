//! What an `answer` of a question that waits is held by, and the moves that clear it.
use super::*;

/// What one question `answer` waits on, as the refusal of `answer` names it:
/// `Unanswered` with the owner's answer move once the question is asked and
/// stands as asked, `Asking` while it is not (its `moves` are what
/// `of_unasked_question` names for it), a question with no body, or a
/// refused question the plan has to drop or replace.
pub enum Waits {
    Unanswered {
        heading: String,
        answer: Move,
    },
    Asking {
        heading: String,
        report: Report,
    },
    /// No decision yet and no question to answer first: the chain it waits
    /// on ends on `refused`, a refused question.
    Stuck {
        heading: String,
        refused: String,
    },
    /// No decision yet and no body written.
    Unwritten {
        heading: String,
        report: Report,
    },
    RefusedTarget {
        heading: String,
    },
    RefusedTier {
        heading: String,
    },
}

/// `answer` (a letter or a refusal) of a question that waits: it stands on a
/// question of its file with no decision, or a question of a lower tier is
/// refused (per "Does answer take a question standing on a question that has no decision?" and
/// "Does a refused question hold the tiers above it?"). Every question it waits on is named, each
/// with the move that clears it: the agent's moves first, and the one owner move last.
pub fn answer_waits(
    heading: &str,
    rel: &str,
    waits: Vec<Waits>,
) -> (crate::msg::Said, Vec<Finding>) {
    let mut parts = Vec::new();
    let mut answer: Option<Move> = None;
    let mut refused: Vec<String> = Vec::new();
    let mut found: Vec<Finding> = Vec::new();
    let mut agent: Vec<Move> = Vec::new();
    let mut owner: Option<Move> = None;
    let mut unwritten: Vec<String> = Vec::new();
    let mut asking: Vec<String> = Vec::new();
    let add = |report: Report,
               found: &mut Vec<Finding>,
               agent: &mut Vec<Move>,
               owner: &mut Option<Move>| {
        for f in &report.findings {
            if !found.iter().any(|g| g.text() == f.text()) {
                found.push(f.clone());
            }
        }
        for m in report.moves {
            if m.actor() == Mover::Owner {
                owner.get_or_insert(m);
            } else if !agent.contains(&m) {
                agent.push(m);
            }
        }
    };
    let refuse_on = |h: String, refused: &mut Vec<String>| {
        if !refused.contains(&h) {
            refused.push(h);
        }
    };
    for w in waits {
        match w {
            Waits::Stuck {
                heading: h,
                refused: r,
            } => {
                parts.push(format!(
                    "\"{}\" has no decision yet and waits on the refused \"{}\"",
                    h, r
                ));
                refuse_on(r, &mut refused);
            }
            Waits::Unwritten { heading: h, report } => {
                parts.push(format!(
                    "\"{}\" has no decision yet and is [{}]",
                    h,
                    QuestionStatus::NoBody.name()
                ));
                unwritten.push(h.clone());
                add(report, &mut found, &mut agent, &mut owner);
            }
            Waits::Asking { heading: h, report } => {
                parts.push(format!("\"{}\" has no decision yet", h));
                asking.push(h.clone());
                add(report, &mut found, &mut agent, &mut owner);
            }
            Waits::Unanswered {
                heading: h,
                answer: m,
            } => {
                parts.push(format!("\"{}\" has no decision yet", h));
                answer = answer.or(Some(m));
            }
            Waits::RefusedTarget { heading: h } => {
                parts.push(format!(
                    "\"{}\" is refused and the question stands on it",
                    h
                ));
                refuse_on(h, &mut refused);
            }
            Waits::RefusedTier { heading: h } => {
                parts.push(format!("\"{}\" is refused and holds the tiers above it", h));
                refuse_on(h, &mut refused);
            }
        }
    }
    // One owner move, last: the plan's approval when a refused question has
    // to go or the plan waits for it, else the first answer owed.
    let mut clauses: Vec<Next> = Vec::new();
    clauses.extend(agent.iter().map(Move::clause));
    if !refused.is_empty() {
        clauses.push(Next::agent(Act::Writes(format!(
            "the plan change that drops or replaces {}",
            quoted(&refused)
        ))));
        owner = Some(Move::new(MoveKind::Approve {
            path: rel.to_string(),
        }));
    }
    if owner.is_none() {
        owner = answer;
    }
    clauses.extend(owner.map(|m| m.clause()));
    if clauses.is_empty() {
        // No move of the questions it waits on: the act on each of them.
        if !unwritten.is_empty() {
            clauses.push(Next::agent(Act::Writes(format!(
                "the body of {}",
                quoted(&unwritten)
            ))));
        }
        if !asking.is_empty() {
            clauses.push(Next::agent(Act::Rewrites(quoted(&asking))));
        }
    }
    // What the rules find in the files it waits on is returned apart: each
    // finding prints whole on a line of its own after the refusal's line.
    let rule = if refused.is_empty() {
        &crate::rule::ANSWER_NEEDS_LOWER_ANSWERS
    } else {
        &crate::rule::ANSWER_NEEDS_NO_REFUSED_BELOW
    };
    (
        crate::msg::said(
            rule,
            format!("\"{}\" waits: {}", heading, parts.join(", ")),
            Next::all(clauses),
        ),
        found,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answer_waits_merges_the_agent_moves_and_puts_the_owner_last() {
        let ask = |_h: &str| Move::new(MoveKind::Plan { rel: "x.md".into() });
        let (said, _) = answer_waits(
            "heading",
            "x.md",
            vec![
                Waits::Asking {
                    heading: "a".into(),
                    report: Report {
                        stops: Vec::new(),
                        findings: Vec::new(),
                        moves: vec![ask("a")],
                    },
                },
                Waits::Asking {
                    heading: "b".into(),
                    report: Report {
                        stops: Vec::new(),
                        findings: Vec::new(),
                        moves: vec![
                            ask("b"),
                            Move::new(MoveKind::Approve {
                                path: "x.md".into(),
                            }),
                        ],
                    },
                },
            ],
        );
        assert_eq!(
            said.found,
            "\"heading\" waits: \"a\" has no decision yet, \"b\" has no decision yet"
        );
        assert_eq!(
            said.next.render(),
            "agent runs: adr plan x.md, owner runs: adr approve x.md"
        );
    }
}
