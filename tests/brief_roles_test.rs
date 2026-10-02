// What the brief tells an agent about who builds and who reviews.
const BRIEF: &str = include_str!("../assets/brief.md");

fn flat() -> String {
    BRIEF.split_whitespace().collect::<Vec<_>>().join(" ")
}

// adr: 9ba58b6a-bc85-44a5-a720-d8e7a6ff7b31
#[test]
fn the_brief_has_another_agent_build_what_an_agent_asked() {
    let b = flat();
    assert!(b.contains("Never build a decision whose question you wrote: another agent builds it."), "{}", b);
    assert!(b.contains("If you wrote the question, stop: another agent builds its decision from the record alone."), "{}", b);
}

// adr: 125ee772-d011-4dab-b53e-97a09ec03198
#[test]
fn the_brief_has_a_review_name_a_decision_for_each_behaviour() {
    let b = flat();
    assert!(
        b.contains("When you review a build, name the decision behind each behaviour. A behaviour with no decision behind it is a finding."),
        "{}",
        b
    );
}
