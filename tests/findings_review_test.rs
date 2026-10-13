// Regressions a review found on the findings build: a stop on a command's own
// input stays a stop line and still holds back the owner's moves; moves keep
// their legitimate alternative acts; messages keep the detail they print.
mod common;

use common::*;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

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

// adr: 1af76f91-5644-42ef-97ea-8f70f934c26c
//
// An origin that cannot be read as text stops the approve of its blame file;
// show prints the stop and does not offer the owner's approve.
#[test]
fn show_of_a_blame_file_prints_an_unreadable_origin_and_offers_no_approve() {
    let dir = blame_setup();
    std::fs::create_dir_all(dir.join("docs")).unwrap();
    std::fs::write(dir.join("docs/x.md"), [0xffu8, 0xfe, 0x00, 0xff]).unwrap();
    let out = run_git(&dir, &["hash-object", "-w", "docs/x.md"]);
    let blob = stdout(&out).trim().to_string();
    write_blame_file(&dir, "adrdom/blame/x.md", "docs/x.md", &blob, "1-1\tdetail\n");

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

// adr: 343d0d28-2c5d-40fc-99aa-22bfba4ea79f
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

/// A running `adr serve`, killed when it goes out of scope.
struct Page {
    child: Child,
    port: u16,
    token: String,
}

impl Drop for Page {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

impl Page {
    fn start(dir: &std::path::Path) -> Page {
        let mut child = scrubbed(Command::new(adr_bin()))
            .current_dir(dir)
            .args(["serve", "--port", "0"])
            .env_remove("CLAUDECODE")
            .env("ADR_OWNER_BYPASS", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("adr serve should spawn");
        let mut line = String::new();
        BufReader::new(child.stdout.as_mut().unwrap()).read_line(&mut line).unwrap();
        let rest = line.trim().strip_prefix("open: http://127.0.0.1:").expect("open line");
        let (port, token) = rest.split_once("/?t=").expect("port and token");
        Page { port: port.parse().unwrap(), token: token.to_string(), child }
    }

    /// The lines of the page at `/`.
    fn lines(&self) -> Vec<String> {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let req = format!("GET / HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\nCookie: adr_t={}\r\n\r\n", self.token);
        stream.write_all(req.as_bytes()).unwrap();
        let mut raw = String::new();
        stream.read_to_string(&mut raw).unwrap();
        let body = raw.split_once("\r\n\r\n").map_or("", |(_, b)| b);
        page_lines(body)
    }
}

const TWO_BARE: &str = r#"# Example

### Is the second one passing?

Problem: whether it passes.

Standard: clear yes or no.

- A) Yes it passes.
  Why: reasons.
  Costs: costs.
- B) No it does not.
  Why: other reasons.
  Costs: other costs.

### Is this one passing?

Problem: whether it passes.

Standard: clear yes or no.

- A) Yes it passes.
  Why: reasons.
  Costs: costs.
- B) No it does not.
  Why: other reasons.
  Costs: other costs.
"#;

const REL: &str = "adrdom/decisions/example.md";

fn draft_repo(record: &str) -> std::path::PathBuf {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    write_record(&dir, "adrdom/terms.md", TERMS_PASS);
    approve_record(&dir, "adrdom/terms.md");
    write_record(&dir, REL, record);
    dir
}

fn all_lines(out: &std::process::Output) -> Vec<String> {
    format!("{}\n{}", stdout(out), stderr(out)).lines().map(str::to_string).collect()
}

fn assert_no_repeats(lines: &[String], what: &str) {
    for l in lines.iter().filter(|l| l.contains("; ")) {
        assert_eq!(lines.iter().filter(|x| *x == l).count(), 1, "{}: {}\n{}", what, l, lines.join("\n"));
    }
}

// adr: 343d0d28-2c5d-40fc-99aa-22bfba4ea79f
//
// A finding line prints once on standard error and on the page too.
#[test]
fn identical_finding_lines_print_once_on_standard_error_and_on_the_page() {
    let twice = format!("{}\n{}", PASS_RECORD.replace("depends: pass\n", ""), PASS_RECORD.replace("# Example\n\n", "").replace("depends: pass\n", ""));
    let dir = draft_repo(&twice);
    let ask = adr(&dir, &["ask", REL], &[], "");
    let err: Vec<String> = stderr(&ask).lines().map(str::to_string).collect();
    assert!(err.iter().any(|l| l.contains("has no depends line")), "{:?}", err);
    assert_no_repeats(&err, "ask");
    let page = Page::start(&dir).lines();
    assert!(page.iter().any(|l| l.contains("has no depends line")), "{:?}", page);
    assert_no_repeats(&page, "page");
}

// adr: 343d0d28-2c5d-40fc-99aa-22bfba4ea79f
//
// The moves of a stop that lists findings, and of a failed read or write,
// name an act on a thing, never "clears the findings below", "fixes what the
// error names" or "writes it".
#[test]
fn generic_moves_name_an_act_on_a_thing() {
    use adr::moves::{answer_waits, Report, Waits};
    use adr::msg;
    use adr::rules::Finding;
    let held = Report {
        stops: Vec::new(),
        findings: vec![Finding::new(&adr::rule::TERMS_FILE_EXISTS, "x".to_string(), "agent y".to_string())],
        moves: Vec::new(),
    };
    let said = [
        msg::not_asked_yet("H?", "adrdom/decisions/a b.md", &held),
        msg::not_written("H?", &held),
        msg::plan_not_approved("x.md", adr::status::FileStatus::New, &held).text(),
        msg::context_not_approved("x.md", adr::status::FileStatus::New, &held).text(),
        msg::terms_missing().text(),
        msg::update_failed("adrdom/ledger.tsv", "boom", msg::after_fix_agent_runs(adr::moves::Cmd::fixed("adr ask x.md"))),
        msg::read_failed("x.md", "boom", msg::after_fix_owner_runs(adr::moves::Cmd::fixed("adr approve x.md"))),
        msg::stdin_read_failed(msg::after_fix_owner_runs(adr::moves::Cmd::fixed("adr unban"))),
        msg::hooks_path_failed("set", msg::after_fix_agent_runs(adr::moves::Cmd::fixed("adr install"))),
    ];
    let wants = [
        "agent runs: adr ask 'adrdom/decisions/a b.md'",
        "agent writes the body of \"H?\"",
        "owner runs: adr approve x.md",
        "owner runs: adr approve x.md",
        "agent writes adrdom/terms.md, owner runs: adr approve adrdom/terms.md",
        "agent fixes adrdom/ledger.tsv, then runs: adr ask x.md",
        "agent fixes x.md, owner runs: adr approve x.md",
        "agent fixes what is piped to adr, owner runs: adr unban",
        "agent fixes the git configuration of core.hooksPath, then runs: adr install",
    ];
    for (got, want) in said.iter().zip(wants) {
        assert!(got.ends_with(want), "{} should end with {}", got, want);
        for bad in ["clears the findings below", "fixes what the error names", "writes it", "\u{1}"] {
            assert!(!got.contains(bad), "{}", got);
        }
    }
    let waits = answer_waits(
        "H?",
        "x.md",
        vec![
            Waits::Unwritten { heading: "A?".into(), report: held.clone() },
            Waits::Asking { heading: "B?".into(), report: held.clone() },
        ],
    );
    assert!(waits.0.next.contains("agent writes the body of \"A?\", agent rewrites \"B?\""), "{}", waits.0.next);
    assert!(!waits.0.next.contains("clears the findings"), "{}", waits.0.next);
}

// adr: 343d0d28-2c5d-40fc-99aa-22bfba4ea79f
//
// The same findings print in the same order under check, show, plan, ask,
// approve and on the page.
#[test]
fn the_same_findings_print_in_the_same_order_in_every_command_and_on_the_page() {
    let dir = draft_repo(TWO_BARE);
    let shared = |lines: &[String]| -> Vec<String> {
        lines.iter().filter(|l| l.contains("has no depends line") || l.contains("uses no term")).cloned().collect()
    };
    let reference = shared(&all_lines(&adr(&dir, &["check"], &[], "")));
    assert_eq!(reference.len(), 4, "{:?}", reference);
    assert!(reference[0].contains("Is the second one passing?") && reference[1].contains("Is the second one passing?"), "{:?}", reference);
    for args in [vec!["show", REL], vec!["plan", REL], vec!["ask", REL]] {
        let got = shared(&all_lines(&adr(&dir, &args, &[], "")));
        assert_eq!(got, reference, "{:?}", args);
    }
    let got = shared(&all_lines(&adr_owner(&dir, &["approve", REL], "")));
    assert_eq!(got, reference, "approve");
    let got = shared(&Page::start(&dir).lines());
    assert_eq!(got, reference, "page");
}
