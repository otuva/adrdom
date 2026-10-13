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
// adr: 9a2ff21b-c20b-4c63-95f2-a7a3cbe236b8
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

// A reader's word stops nothing, a decision changes only through a
// supersedes line, and closes names the option the record forces: so a
// closes or inconsistent mark has the agent strike, supersede or object,
// and the brief says so.
// adr: e78b8297-fd89-4e09-9c3e-9eeb0fd0f654
// adr: dad5273c-afe2-4ab1-a451-e2a5d4d9a26d
// adr: d10dab89-1d42-4764-9d72-22d6fd6a9260
#[test]
fn the_brief_has_the_agent_strike_supersede_or_object_on_a_closes_or_inconsistent_mark() {
    let b = flat();
    assert!(b.contains("Under a neighbour marked closes with a letter the record leaves that option alone standing, so the question is decided: strike the heading, or, when the neighbour is a decision the question is there to change, write it on the `supersedes:` line; when you read the neighbour and find more than one option standing, keep your line and say why in chat."), "{}", b);
    assert!(b.contains("Under a neighbour marked inconsistent, write it on `supersedes:` when it is a decision the question is there to change, reword the question so the two agree, or keep your line and say why in chat."), "{}", b);
}
