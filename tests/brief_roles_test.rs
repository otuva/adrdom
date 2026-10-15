// What the brief tells an agent about who builds and who reviews.
static BRIEF: std::sync::LazyLock<String> = std::sync::LazyLock::new(adr::commands::install::brief);

fn flat() -> String {
    BRIEF.split_whitespace().collect::<Vec<_>>().join(" ")
}

// adr: 9ba58b6a-bc85-44a5-a720-d8e7a6ff7b31
#[test]
fn the_brief_has_another_agent_build_what_an_agent_asked() {
    let b = flat();
    assert!(b.contains("it does not build the decisions it asked"), "{}", b);
    assert!(b.contains("A builder is another session; it builds from the record alone"), "{}", b);
}

// adr: 125ee772-d011-4dab-b53e-97a09ec03198
#[test]
fn the_brief_has_a_review_name_a_decision_for_each_behaviour() {
    let b = flat();
    assert!(b.contains("In a review of a build, a behaviour with no decision behind it is a finding."), "{}", b);
}

// adr: 6c406624-b874-4697-9a44-91941019b7d1
// adr: db07a54d-4d80-4035-a9ef-eabcc0c872b3
// adr: 9a2ff21b-c20b-4c63-95f2-a7a3cbe236b8
#[test]
fn the_brief_tells_a_planner_what_the_tool_holds_it_from() {
    let b = flat();
    assert!(b.contains("A session that runs `adr plan` or `adr ask` is a planner until it ends"), "{}", b);
    assert!(b.contains("its commits hold only paths under `adrdom/`"), "{}", b);
    assert!(b.contains("`builder` (another session)"), "{}", b);
}
