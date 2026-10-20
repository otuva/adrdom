mod common;
use common::*;

// What the brief tells an agent about who builds and who reviews.
static BRIEF: std::sync::LazyLock<String> = std::sync::LazyLock::new(adr::commands::install::brief);

fn flat() -> String {
    BRIEF.split_whitespace().collect::<Vec<_>>().join(" ")
}

// adr: 9ba58b6a-bc85-44a5-a720-d8e7a6ff7b31
#[test]
fn the_brief_has_another_agent_build_what_an_agent_asked() {
    let b = flat();
    assert!(
        b.contains("it does not build the decisions it asked"),
        "{}",
        b
    );
    assert!(
        b.contains("A builder is another session; it builds from the record alone"),
        "{}",
        b
    );
}

// adr: 125ee772-d011-4dab-b53e-97a09ec03198
#[test]
fn the_brief_has_a_review_name_a_decision_for_each_behaviour() {
    let b = flat();
    assert!(
        b.contains("In a review of a build, name the decision behind each behaviour the diff adds or changes; a behaviour with no decision behind it is a finding."),
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
    assert!(
        b.contains("A session that runs `adr plan` or `adr ask` is a planner until it ends"),
        "{}",
        b
    );
    assert!(
        b.contains("its commits hold only paths under `adrdom/`"),
        "{}",
        b
    );
    assert!(
        b.contains("- `builder`: another session's; leave it."),
        "{}",
        b
    );
}

// adr: 28b5837d-46ae-459e-ba4c-12265ef07ac1
#[test]
fn the_brief_says_the_option_sentence_binds() {
    let b = flat();
    assert!(b.contains("A decision binds by its option sentence; Why and Costs explain it, and on a clash the option holds."), "{}", b);
}

// adr: 1259323f-cdec-4101-9d47-a165a286089f
#[test]
fn the_brief_names_the_adr_code_comment() {
    let b = flat();
    assert!(b.contains("may carry the comment `adr-code: <id>`, found by search; it counts as no test and no rule judges it."), "{}", b);
}

// adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
// adr: 5c399a14-1e5c-48eb-b502-b57524c01c24
// adr: 69708a07-469c-406b-9e25-63da1605b432
#[test]
fn the_brief_gives_the_shape_of_a_blame_file() {
    let b = flat();
    assert!(b.contains("A row is a line range, a tab, and one word: `decision` marks a range the owner settles by picking an option, `detail` every other range.") && b.contains("The ranges reach every line of the origin"), "{}", b);
    assert!(
        b.contains("with a blame line under the heading: `blame: old-file 10-14 23-30, other 3-9`"),
        "{}",
        b
    );
}

// adr: aa275fac-def5-42a2-8a61-660142acad8e
#[test]
fn the_commands_the_brief_calls_the_owners_refuse_an_agent() {
    let b = flat();
    assert!(
        b.contains("The owner's commands are `adr approve`, `adr answer`, `adr unban`, `adr serve` and `adr uninstall`; they refuse an agent."),
        "{}",
        b
    );
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    for args in [
        vec!["approve", "adrdom/terms.md"],
        vec!["answer", "00000000-0000-0000-0000-000000000000@1", "A"],
        vec!["unban"],
        vec!["serve"],
        vec!["uninstall"],
    ] {
        let out = adr(&dir, &args, &[], "");
        assert!(!out.status.success(), "{:?} ran for an agent", args);
        let err = stderr(&out);
        assert!(err.contains("owner's command"), "{:?}: {}", args, err);
    }
}
