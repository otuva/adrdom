//! What is next in a topic file, and the lines of the `show` listing.

// --- what is next in a topic file (`moves::of_topic_file`) ----------------
// One line per move, each opening with who makes it, in the words a
// refusal uses. `show` prints them under the file, `approve` when it has
// nothing to ask.

// --- show: the lines of its listing ----------------------------------------
// Labels and layout, no refusal among them: a note carries no `Next`.

/// The header of a group of findings on a terminal (per "What does the tool
/// add to the shape of what it prints to a terminal?", f3c9e736 A).
pub fn group_findings() -> String {
    "Findings".to_string()
}

/// The header of a group of moves on a terminal.
pub fn group_moves() -> String {
    "Moves".to_string()
}

pub fn show_topic(topic: &str, rel: &str) -> String {
    if topic.is_empty() {
        return format!("# ({})", rel);
    }
    format!("# {} ({})", topic, rel)
}

/// A question of another topic file, as `show --dependents` names it.
pub fn show_question_of(heading: &str, rel: &str) -> String {
    format!("{} ({})", heading, rel)
}

/// Under a question not yet asked: no neighbour of its heading and no word of
/// the reader on its body is on record.
/// The move that reads them is the file's own (`moves::of_planning`), not
/// computed here a second time.
pub fn show_no_records() -> String {
    "neighbours: none on record yet".to_string()
}

/// After the rows of a blame file under `show --questions`: how many of its
/// `decision` rows a question names and how many none names (per "Does check
/// report a decision row that no question names?", ee27fd4c A).
pub fn show_decision_rows(with: usize, without: usize) -> String {
    format!(
        "decision rows: {} with a question, {} with none",
        with, without
    )
}

/// Above the lines of a changed part whose approved text no commit holds:
/// every line is shown as added, and why.
pub fn show_no_baseline() -> String {
    "the approved text is in no commit: every line shows as added".to_string()
}

/// Above the questions struck from a plan whose approved version no commit
/// holds, when the ledger alone cannot tell which of them the latest plan
/// held.
pub fn show_struck_unsure() -> String {
    "the approved plan is in no commit: every question approved for this file and gone from it is named, some may have left by an earlier approval".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // adr: a469ecb9-a4f1-4a2a-89c5-7378cc1138b4
    //
    // The words of the count `show --questions` prints.
    #[test]
    fn the_count_of_decision_rows_reads_word_for_word() {
        assert_eq!(
            show_decision_rows(1, 2),
            "decision rows: 1 with a question, 2 with none"
        );
    }
}
