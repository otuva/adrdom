// Regressions a review found on the findings build: a stop on a command's own
// input stays a stop line and still holds back the owner's moves; moves keep
// their legitimate alternative acts; messages keep the detail they print.
mod common;

use common::*;

const PASS_RECORD: &str = r#"# Example

### 1.1 Is this pass?
depends: pass

Problem: whether it passes.

Standard: clear yes or no.

- A) Yes it passes.
  Why: reasons.
  Costs: costs.
- B) No it does not.
  Why: other reasons.
  Costs: other costs.
"#;

const TERMS_PASS: &str = "# Terms\n\n\
## pass\n\
id: 11111111-1111-1111-1111-111111111111\n\
definition: Whether something is accepted.\n\
aliases: \n";

// adr: aa475aa3-05b2-429a-b33a-46b1dcb52049
//
// A terms file that cannot be read stops approve, ask and answer; show of a
// topic file prints that stop as the stop line it is and offers none of the
// moves those commands would refuse.
#[test]
fn show_of_a_topic_file_prints_an_unreadable_terms_file_and_offers_no_approve() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    write_record(&dir, "adrdom/terms.md", TERMS_PASS);
    approve_record(&dir, "adrdom/terms.md");
    write_record(&dir, "adrdom/decisions/example.md", PASS_RECORD);
    // The terms file is not text.
    std::fs::write(dir.join("adrdom/terms.md"), [0xffu8, 0xfe, 0x00, 0xff]).unwrap();

    let out = adr(&dir, &["show", "adrdom/decisions/example.md"], &[], "");
    let text = format!("{}{}", stdout(&out), stderr(&out));
    assert!(text.contains("show: ") && text.contains("adrdom/terms.md"), "{}", text);
    assert!(!text.contains("owner runs: adr approve adrdom/decisions/example.md"), "{}", text);
    assert!(!text.contains("adr ask"), "{}", text);
}

// adr: 848e1b76-9cfe-483b-94dd-094eb9b2f995
//
// An origin that cannot be read as text stops the approve of its blame file;
// show prints the stop and does not offer the owner's approve.
#[test]
fn show_of_a_blame_file_prints_an_unreadable_origin_and_offers_no_approve() {
    let (dir, _id1, _id2) = blame_setup();
    std::fs::create_dir_all(dir.join("docs")).unwrap();
    std::fs::write(dir.join("docs/x.md"), [0xffu8, 0xfe, 0x00, 0xff]).unwrap();
    let out = run_git(&dir, &["hash-object", "-w", "docs/x.md"]);
    let blob = stdout(&out).trim().to_string();
    write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, "1-1\tdropped: dead\n");

    let out = adr(&dir, &["show", "adrdom/blame/x.md"], &[], "");
    let text = format!("{}{}", stdout(&out), stderr(&out));
    assert!(text.contains("cannot be read as text"), "{}", text);
    assert!(text.contains("agent fixes docs/x.md"), "{}", text);
    assert!(!text.contains("owner runs: adr approve adrdom/blame/x.md"), "{}", text);
}

// adr: 7c306235-6243-4d79-b9aa-08ed19662779
//
// A word used in two entries names where both entries are, and a repeated
// identifier is its own rule, not a missing field.
#[test]
fn the_terms_file_errors_name_both_entries_and_the_repeated_identifier_rule() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    write_record(
        &dir,
        "adrdom/terms.md",
        "# Terms\n\n## one\nid: 11111111-1111-1111-1111-111111111111\ndefinition: A thing.\naliases: shared\n\n## two\nid: 22222222-2222-2222-2222-222222222222\ndefinition: Another thing.\naliases: shared\n",
    );
    let text = format!("{}{}", stdout(&adr(&dir, &["check"], &[], "")), stderr(&adr(&dir, &["check"], &[], "")));
    assert!(text.contains("used in both \"one\" (line 3) and \"two\" (line 8)"), "{}", text);

    write_record(
        &dir,
        "adrdom/terms.md",
        "# Terms\n\n## one\nid: 11111111-1111-1111-1111-111111111111\ndefinition: A thing.\naliases: \n\n## two\nid: 11111111-1111-1111-1111-111111111111\ndefinition: Another thing.\naliases: \n",
    );
    let text = format!("{}{}", stdout(&adr(&dir, &["check"], &[], "")), stderr(&adr(&dir, &["check"], &[], "")));
    assert!(text.contains("duplicate id"), "{}", text);
    assert!(text.contains("one identifier names one term"), "{}", text);
    assert!(!text.contains("every term has an identifier, name, definition and aliases"), "{}", text);
}

// adr: 184b5bcb-fa72-436b-92cb-4d4605366c59
//
// A heading written twice in a topic file prints each finding once.
#[test]
fn identical_finding_lines_print_once() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    write_record(&dir, "adrdom/terms.md", TERMS_PASS);
    approve_record(&dir, "adrdom/terms.md");
    let twice = format!("{}\n{}", PASS_RECORD.replace("depends: pass\n", ""), PASS_RECORD.replace("# Example\n\n", "").replace("depends: pass\n", ""));
    write_record(&dir, "adrdom/decisions/example.md", &twice);
    for args in [vec!["check"], vec!["show", "adrdom/decisions/example.md"]] {
        let out = adr(&dir, &args, &[], "");
        let text = format!("{}{}", stdout(&out), stderr(&out));
        let lines: Vec<&str> = text.lines().filter(|l| l.contains("has no depends line")).collect();
        let mut uniq = lines.clone();
        uniq.dedup();
        assert!(!lines.is_empty(), "{:?}: {}", args, text);
        for l in &lines {
            assert_eq!(lines.iter().filter(|x| *x == l).count(), 1, "{:?}: {}", args, text);
        }
    }
}
