use adr::tells::{CODE_STATE, PROVENANCE};

const BRIEF: &str = include_str!("../assets/brief.md");

// The agent names sit in the provenance list but the brief says "the name of
// an agent" instead: the records never name one.
const AGENT_NAMES: &[&str] = &["claude", "codex", "gemini", "copilot"];

// The quoted phrases of the brief's refused-words section, in order.
fn refused_words_in_brief() -> Vec<String> {
    let start = BRIEF.find("Three kinds of words are refused").unwrap();
    let end = BRIEF[start..].find("- a date.").unwrap() + start;
    let section = BRIEF[start..end].replace("\n  ", " ");
    let mut out = Vec::new();
    let mut parts = section.split('"');
    parts.next();
    while let (Some(phrase), Some(_)) = (parts.next(), parts.next()) {
        out.push(phrase.to_string());
    }
    out
}

// The brief names the phrases the tool refuses in a question. It is copied
// by hand, so this keeps it equal to the lists in src/tells.rs, in their
// order, agent names left out.
#[test]
fn brief_lists_exactly_the_phrases_the_tool_refuses() {
    // adr: 149c9a73-16eb-46aa-9806-2950423ee1a2
    let mut expected: Vec<String> = CODE_STATE.iter().map(|s| s.to_string()).collect();
    expected.extend(
        PROVENANCE
            .iter()
            .filter(|p| !AGENT_NAMES.contains(p))
            .map(|s| s.to_string()),
    );
    assert_eq!(refused_words_in_brief(), expected);
}
