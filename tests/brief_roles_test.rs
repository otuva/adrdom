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

// adr: 6c406624-b874-4697-9a44-91941019b7d1
// adr: db07a54d-4d80-4035-a9ef-eabcc0c872b3
// adr: 8f81dee1-9c19-447d-baa8-d7d0af691db8
#[test]
fn the_brief_tells_a_planner_what_the_tool_holds_it_from() {
    let b = flat();
    assert!(b.contains("A session that runs `adr plan` or `adr ask` on a topic file is a planner until the session ends."), "{}", b);
    assert!(b.contains("In a planner's session every build move opens with `builder`, and check refuses a commit that holds a path outside `adrdom/`: a builder commits it."), "{}", b);
    assert!(b.contains("A line that opens with `builder` is for a builder, another session. Leave it."), "{}", b);
}

// adr: 2381d759-f85f-484e-9031-fadbe6ea045a
#[test]
fn the_brief_has_the_builder_move_a_tie_that_names_a_decision_not_current() {
    let b = flat();
    assert!(b.contains("It reports a test whose `adr: <id>` names a decision that is not current, and fails nothing: the builder points the tie at the decision that took its place, or removes it."), "{}", b);
}
