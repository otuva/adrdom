// What binds in an answered question is the option sentence; the tool shows
// only that sentence under a decision, never the Why or Costs lines.
mod common;

use common::*;

const QUESTION: &str = r#"# Example

### What does the authority hold?
depends: authority

Problem: an authority is unclear.

Standard: plain words.

- A) The authority holds the lid.
  Why: zebrafruit reasoning.
  Costs: quokkaburden price.
- B) The authority holds nothing.
  Why: marmotsong reasoning.
  Costs: lynxcloud price.
"#;

const WORDS: [&str; 4] = ["zebrafruit", "quokkaburden", "marmotsong", "lynxcloud"];

#[test]
fn show_and_answer_print_the_picked_option_sentence_and_not_why_or_costs() {
    // adr: 28b5837d-46ae-459e-ba4c-12265ef07ac1
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "authority");
    let record = write_record(&dir, "adrdom/decisions/example.md", QUESTION);
    approve_record(&dir, "adrdom/decisions/example.md");
    let id = extract_ids(&std::fs::read_to_string(&record).unwrap())[0].clone();

    let ans = adr_answer(&dir, &id, "A");
    assert!(ans.status.success(), "answer failed: {}", stderr(&ans));
    let said = stdout(&ans);
    assert!(said.contains("What does the authority hold?"), "{}", said);
    assert!(said.contains("The authority holds the lid."), "{}", said);
    for w in WORDS {
        assert!(!said.contains(w), "answer printed {}: {}", w, said);
    }
    assert!(!said.contains("The authority holds nothing."), "{}", said);

    let show = stdout(&adr(&dir, &["show"], &[], ""));
    assert!(show.contains("What does the authority hold?"), "{}", show);
    assert!(show.contains("The authority holds the lid."), "{}", show);
    for w in WORDS {
        assert!(!show.contains(w), "show printed {}: {}", w, show);
    }
}
