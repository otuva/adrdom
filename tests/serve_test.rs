mod common;

use common::*;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

/// Mirrors `common::adr_owner`'s env, but spawns the process (rather than
/// running it to completion) since `adr serve` never exits on its own; the
/// caller reads its `open:` line off stdout and kills it when done.
fn spawn_owner(dir: &Path, args: &[&str]) -> Child {
    Command::new(adr_bin())
        .current_dir(dir)
        .args(args)
        .env_remove("CLAUDECODE")
        .env("ADR_OWNER_BYPASS", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("adr serve should spawn")
}

/// Reads the single `open: http://127.0.0.1:<port>/?t=<token>` line off the
/// child's stdout and returns (port, token).
fn read_open_line(child: &mut Child) -> (u16, String) {
    let stdout = child.stdout.as_mut().expect("piped stdout");
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    reader.read_line(&mut line).expect("read open line");
    let line = line.trim();
    let prefix = "open: http://127.0.0.1:";
    assert!(line.starts_with(prefix), "unexpected open line: {}", line);
    let rest = &line[prefix.len()..];
    let (port_str, token_part) = rest.split_once('/').expect("path after port");
    let port: u16 = port_str.parse().expect("port");
    let token = token_part
        .strip_prefix("?t=")
        .expect("token query")
        .to_string();
    (port, token)
}

struct HttpResponse {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
}

fn raw_request(port: u16, request: &str) -> HttpResponse {
    let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("connect");
    stream
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    stream.write_all(request.as_bytes()).unwrap();
    stream.flush().unwrap();
    let mut raw = String::new();
    stream.read_to_string(&mut raw).unwrap();

    let (head, body) = raw.split_once("\r\n\r\n").unwrap_or((raw.as_str(), ""));
    let mut lines = head.lines();
    let status_line = lines.next().unwrap_or("");
    let status: u16 = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    let headers = lines
        .filter_map(|l| l.split_once(':'))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect();
    HttpResponse {
        status,
        headers,
        body: body.to_string(),
    }
}

fn header<'a>(resp: &'a HttpResponse, name: &str) -> Option<&'a str> {
    resp.headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

fn get(port: u16, path: &str, cookie: Option<&str>) -> HttpResponse {
    let mut req = format!(
        "GET {} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n",
        path
    );
    if let Some(c) = cookie {
        req.push_str(&format!("Cookie: {}\r\n", c));
    }
    req.push_str("\r\n");
    raw_request(port, &req)
}

fn post(port: u16, path: &str, cookie: Option<&str>, form: &str) -> HttpResponse {
    let mut req = format!(
        "POST {} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n",
        path
    );
    if let Some(c) = cookie {
        req.push_str(&format!("Cookie: {}\r\n", c));
    }
    req.push_str("Content-Type: application/x-www-form-urlencoded\r\n");
    req.push_str(&format!("Content-Length: {}\r\n\r\n", form.len()));
    req.push_str(form);
    raw_request(port, &req)
}

/// Sets up a repo with `adr install`, the fixture record approved, one
/// `ask` run (so 1.1 is Open), and returns the repo dir.
fn setup_repo_with_open_question() -> std::path::PathBuf {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "decision");
    let record = write_record(&dir, "adrdom/decisions/example.md", EXAMPLE_RECORD);
    approve_via_ledger(&dir, "adrdom/decisions/example.md", EXAMPLE_RECORD);
    let ask_out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(ask_out.status.success(), "ask failed: {}", stderr(&ask_out));
    dir
}

#[test]
fn get_root_without_cookie_is_forbidden() {
    let dir = setup_repo_with_open_question();
    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, _token) = read_open_line(&mut child);

    let resp = get(port, "/", None);
    assert_eq!(resp.status, 403, "body: {}", resp.body);

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn get_root_with_token_sets_cookie_and_redirects() {
    let dir = setup_repo_with_open_question();
    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);

    let resp = get(port, &format!("/?t={}", token), None);
    assert_eq!(resp.status, 302, "body: {}", resp.body);
    let set_cookie = header(&resp, "Set-Cookie").expect("Set-Cookie header");
    assert!(set_cookie.contains("adr_t="), "{}", set_cookie);
    assert!(set_cookie.contains("HttpOnly"), "{}", set_cookie);

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn get_root_with_cookie_shows_fixture_topic() {
    let dir = setup_repo_with_open_question();
    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);

    let cookie = format!("adr_t={}", token);
    let resp = get(port, "/", Some(&cookie));
    assert_eq!(resp.status, 200, "body: {}", resp.body);
    assert!(resp.body.contains("Example"), "body: {}", resp.body);

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn get_terms_without_cookie_is_forbidden() {
    let dir = setup_repo_with_open_question();
    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, _token) = read_open_line(&mut child);

    let resp = get(port, "/terms", None);
    assert_eq!(resp.status, 403, "body: {}", resp.body);

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn nav_has_terms_link() {
    let dir = setup_repo_with_open_question();
    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let cookie = format!("adr_t={}", token);

    let resp = get(port, "/", Some(&cookie));
    assert_eq!(resp.status, 200, "body: {}", resp.body);
    assert!(
        resp.body.contains("<a href=\"/terms\">Terms</a>"),
        "body: {}",
        resp.body
    );

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn post_answer_with_cookie_and_token_answers_and_redirects() {
    let dir = setup_repo_with_open_question();
    let content = std::fs::read_to_string(dir.join("adrdom/decisions/example.md")).unwrap();
    let ids = extract_ids(&content);
    let question_id = ids[0].clone();

    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let cookie = format!("adr_t={}", token);

    let form = format!("id={}&answer=A&t={}", question_id, token);
    let resp = post(port, "/answer", Some(&cookie), &form);
    assert_eq!(resp.status, 302, "body: {}", resp.body);
    let location = header(&resp, "Location").expect("Location header");
    assert!(
        location.starts_with(&format!("/next?after={}", question_id)),
        "{}",
        location
    );

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        ledger
            .lines()
            .any(|l| l.contains("\tanswered\t") && l.contains(question_id.as_str())),
        "ledger missing answered row: {}",
        ledger
    );

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn post_answer_without_t_field_is_forbidden() {
    let dir = setup_repo_with_open_question();
    let content = std::fs::read_to_string(dir.join("adrdom/decisions/example.md")).unwrap();
    let ids = extract_ids(&content);
    let question_id = ids[0].clone();

    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let cookie = format!("adr_t={}", token);

    let form = format!("id={}&answer=A", question_id);
    let resp = post(port, "/answer", Some(&cookie), &form);
    assert_eq!(resp.status, 403, "body: {}", resp.body);

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        !ledger.lines().any(|l| l.contains("\tanswered\t")),
        "answer must not have run: {}",
        ledger
    );

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn get_next_with_cookie_shows_question_heading() {
    let dir = setup_repo_with_open_question();
    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let cookie = format!("adr_t={}", token);

    let resp = get(port, "/next", Some(&cookie));
    assert_eq!(resp.status, 200, "body: {}", resp.body);
    assert!(
        resp.body.contains("What is the unit of authority called?"),
        "body: {}",
        resp.body
    );

    let _ = child.kill();
    let _ = child.wait();
}

// --- terms fixtures (7.9: words, no-term, entries, before/after) -------

const WORDS_TERMS_MD: &str = "# Terms\n\n\
## decision\n\
id: 11111111-1111-1111-1111-111111111111\n\
definition: A choice answered by the owner.\n\
aliases: \n";

const WORDS_RECORD: &str = r#"# Topic

Context prose.

## Group One

### 1.1 What counts as a decision?

Problem: the decision about a frobnicate is unclear.

Standard: decisions must be recorded plainly.

- A) A decision is one commitment.
  Why: reasons.
  Costs: costs.
- B) A decision is a block.
  Why: other reasons.
  Costs: other costs.
"#;

#[test]
fn get_next_shows_question_words_and_no_entry() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let record = write_record(&dir, "adrdom/decisions/example.md", WORDS_RECORD);
    let terms = write_record(&dir, "adrdom/terms.md", WORDS_TERMS_MD);
    assert!(adr_owner(&dir, &["approve", terms.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr_owner(&dir, &["approve", record.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let cookie = format!("adr_t={}", token);

    let resp = get(port, "/next", Some(&cookie));
    assert_eq!(resp.status, 200, "body: {}", resp.body);

    // The aside groups the word under its term name, with the definition.
    assert!(
        resp.body.contains("Terms in this question"),
        "body: {}",
        resp.body
    );
    assert!(resp.body.contains("<dt>decision"), "body: {}", resp.body);
    assert!(
        resp.body.contains("A choice answered by the owner."),
        "body: {}",
        resp.body
    );

    // A coined word with no term shows up in the "No term" line.
    assert!(resp.body.contains("frobnicate"), "body: {}", resp.body);
    // A common word never does.
    let no_entry_start = resp.body.find("No term:").expect("No term line present");
    let no_entry_line = &resp.body[no_entry_start..];
    let no_entry_line = &no_entry_line[..no_entry_line.find("</p>").unwrap_or(no_entry_line.len())];
    assert!(
        !no_entry_line.contains("the"),
        "no-term line: {}",
        no_entry_line
    );
    // Nor does a plural of a paired term ("decisions" resolves the same
    // term as "decision").
    assert!(
        !no_entry_line.contains("decisions"),
        "no-term line: {}",
        no_entry_line
    );

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn get_next_marks_term_words_in_text() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let record = write_record(&dir, "adrdom/decisions/example.md", WORDS_RECORD);
    let terms = write_record(&dir, "adrdom/terms.md", WORDS_TERMS_MD);
    assert!(adr_owner(&dir, &["approve", terms.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr_owner(&dir, &["approve", record.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let cookie = format!("adr_t={}", token);

    let resp = get(port, "/next", Some(&cookie));
    assert_eq!(resp.status, 200, "body: {}", resp.body);

    // "decision" pairs a term everywhere it appears (heading and Problem),
    // wrapped as a titled span rather than plain text.
    assert!(
        resp.body.contains("<span class=\"t\" title=\"decision:"),
        "body: {}",
        resp.body
    );

    let _ = child.kill();
    let _ = child.wait();
}

const UUID_INLINE_RECORD: &str = r#"# Topic

Context prose.

## Group One

### 1.1 What counts as a decision?

Problem: the decision about a frobnicate is unclear.

Standard: decisions must be recorded plainly.

- A) A decision naming a1b2c3d4-e5f6-4a1b-9c3d-112233445566 is one commitment.
  Why: reasons.
  Costs: costs.
- B) A decision is a block.
  Why: other reasons.
  Costs: other costs.
"#;

/// A dashed uuid inline in option text must not leak hex fragments (split
/// at the hyphens by tokenizing) into the "No term" line.
#[test]
fn get_next_no_entry_line_has_no_uuid_fragments() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let record = write_record(&dir, "adrdom/decisions/example.md", UUID_INLINE_RECORD);
    let terms = write_record(&dir, "adrdom/terms.md", WORDS_TERMS_MD);
    assert!(adr_owner(&dir, &["approve", terms.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr_owner(&dir, &["approve", record.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let cookie = format!("adr_t={}", token);

    let resp = get(port, "/next", Some(&cookie));
    assert_eq!(resp.status, 200, "body: {}", resp.body);

    let no_entry_start = resp.body.find("No term:").expect("No term line present");
    let no_entry_line = &resp.body[no_entry_start..];
    let no_entry_line = &no_entry_line[..no_entry_line.find("</p>").unwrap_or(no_entry_line.len())];
    for fragment in ["a1b2c3d4", "e5f6", "4a1b", "9c3d", "112233445566"] {
        assert!(
            !no_entry_line.contains(fragment),
            "no-term line leaked uuid fragment \"{}\": {}",
            fragment,
            no_entry_line
        );
    }

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn terms_article_lists_entries_with_dependents_count() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let record = write_record(&dir, "adrdom/decisions/example.md", WORDS_RECORD);
    let terms = write_record(&dir, "adrdom/terms.md", WORDS_TERMS_MD);
    assert!(adr_owner(&dir, &["approve", terms.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr_owner(&dir, &["approve", record.to_str().unwrap()], "")
        .status
        .success());
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());
    let content = std::fs::read_to_string(&record).unwrap();
    let question_id = extract_ids(&content)[0].clone();
    assert!(adr_owner(&dir, &["answer", &question_id, "A"], "")
        .status
        .success());

    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let cookie = format!("adr_t={}", token);

    // The overview shows only the summary card, with a link to /terms.
    let overview = get(port, "/", Some(&cookie));
    assert_eq!(overview.status, 200, "body: {}", overview.body);
    assert!(
        overview
            .body
            .contains("<span class=\"count\">1 term</span>"),
        "body: {}",
        overview.body
    );
    assert!(
        overview.body.contains("Open the terms"),
        "body: {}",
        overview.body
    );

    // /terms lists the full entry: name, definition, and dependents.
    let resp = get(port, "/terms", Some(&cookie));
    assert_eq!(resp.status, 200, "body: {}", resp.body);
    assert!(resp.body.contains("decision"), "body: {}", resp.body);
    assert!(
        resp.body.contains("A choice answered by the owner."),
        "body: {}",
        resp.body
    );
    assert!(
        resp.body
            .contains("Paired by <a href=\"/decisions\">1 decision</a>"),
        "body: {}",
        resp.body
    );

    let _ = child.kill();
    let _ = child.wait();
}

const WORDS_TERMS_MD_V2: &str = "# Terms\n\n\
## decision\n\
id: 11111111-1111-1111-1111-111111111111\n\
definition: A choice the owner answers in, once, in writing.\n\
aliases: \n";

#[test]
fn terms_article_shows_before_after_when_unapproved() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let terms = write_record(&dir, "adrdom/terms.md", WORDS_TERMS_MD);
    assert!(adr_owner(&dir, &["approve", terms.to_str().unwrap()], "")
        .status
        .success());
    run_git(&dir, &["add", "adrdom/terms.md", "adrdom/state/ledger.tsv"]);
    let commit_out = run_git(&dir, &["commit", "-q", "-m", "approve terms v1"]);
    assert!(commit_out.status.success(), "{:?}", commit_out);

    // Edit the definition without re-approving: HEAD still matches the
    // last `terms` ledger row, so the page can diff against it.
    write_record(&dir, "adrdom/terms.md", WORDS_TERMS_MD_V2);

    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let cookie = format!("adr_t={}", token);

    // The overview shows the terms as not approved.
    let overview = get(port, "/", Some(&cookie));
    assert_eq!(overview.status, 200, "body: {}", overview.body);
    assert!(
        overview.body.contains("not approved"),
        "body: {}",
        overview.body
    );

    let resp = get(port, "/terms", Some(&cookie));
    assert_eq!(resp.status, 200, "body: {}", resp.body);
    assert!(
        resp.body.contains("class=\"changed-box\""),
        "changed-box missing: {}",
        resp.body
    );
    assert!(
        resp.body.contains("Approve the terms"),
        "approve button missing: {}",
        resp.body
    );
    assert!(
        resp.body.contains("A choice answered by the owner."),
        "before text missing: {}",
        resp.body
    );
    assert!(
        resp.body
            .contains("A choice the owner answers in, once, in writing."),
        "after text missing: {}",
        resp.body
    );

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn serve_refuses_when_claudecode_is_set() {
    let dir = setup_repo_with_open_question();
    let out = Command::new(adr_bin())
        .current_dir(&dir)
        .args(["serve", "--port", "0"])
        .env("CLAUDECODE", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("adr serve should run");
    assert_eq!(out.status.code(), Some(2), "stderr: {}", stderr(&out));
    assert!(
        stderr(&out).contains("owner authorization required"),
        "stderr: {}",
        stderr(&out)
    );
}
