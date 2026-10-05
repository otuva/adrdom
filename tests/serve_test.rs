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
    approve_terms_with(&dir, &["decision", "example"]);
    let record = write_record(&dir, "adrdom/decisions/example.md", EXAMPLE_RECORD);
    approve_record(&dir, "adrdom/decisions/example.md");
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

// adr: df452a5a-1e2d-497e-93e9-7d6a80e8540d
//
// "the browser swaps [the token] for a cookie": a `GET` carrying the
// printed `?t=` token gets back a `Set-Cookie: adr_t=<token>` (HttpOnly),
// not a page body.
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

// adr: 11be073c-ce91-44d8-833a-ef0b5e0b1672
//
// "answer takes that version from the terminal and the page alike": the
// page's POST with the current version -- the same hidden `v` field
// `render_next` writes -- answers, same as the terminal.
#[test]
fn post_answer_with_cookie_and_token_answers_and_redirects() {
    let dir = setup_repo_with_open_question();
    let content = std::fs::read_to_string(dir.join("adrdom/decisions/example.md")).unwrap();
    let ids = extract_ids(&content);
    let question_id = ids[0].clone();

    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let cookie = format!("adr_t={}", token);

    let version = current_version(&dir, &question_id);
    let form = format!("id={}&v={}&answer=A&t={}", question_id, version, token);
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

// adr: 11be073c-ce91-44d8-833a-ef0b5e0b1672
//
// "a version that no longer matches refuses the answer", from the page:
// editing the question and re-asking it moves its version; POSTing back
// the stale one refuses and writes nothing, the same as the terminal.
#[test]
fn post_answer_with_a_stale_version_is_refused_and_writes_nothing() {
    let dir = setup_repo_with_open_question();
    let record_path = dir.join("adrdom/decisions/example.md");
    let content = std::fs::read_to_string(&record_path).unwrap();
    let ids = extract_ids(&content);
    let question_id = ids[0].clone();
    let stale_version = current_version(&dir, &question_id);

    // Edit the question's Problem text (inside the hashed range, before any
    // Answer) and re-ask: the section's hash -- and so its version -- moves.
    let edited = content.replace(
        "names leaked into decisions.",
        "names leaked into decisions, badly.",
    );
    std::fs::write(&record_path, edited).unwrap();
    approve_record(&dir, "adrdom/decisions/example.md");
    let reask = adr(&dir, &["ask", record_path.to_str().unwrap()], &[], "");
    assert!(reask.status.success(), "reask failed: {}", stderr(&reask));
    assert_ne!(
        current_version(&dir, &question_id),
        stale_version,
        "the re-ask should have moved the version"
    );

    let before_record = std::fs::read_to_string(&record_path).unwrap();
    let before_ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();

    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let cookie = format!("adr_t={}", token);

    let form = format!("id={}&v={}&answer=A&t={}", question_id, stale_version, token);
    let resp = post(port, "/answer", Some(&cookie), &form);
    assert_eq!(resp.status, 302, "body: {}", resp.body);
    let location = header(&resp, "Location").expect("Location header");
    let followed = get(port, location, Some(&cookie));
    assert_eq!(followed.status, 200, "body: {}", followed.body);
    assert!(
        followed.body.contains("has moved since that version was read"),
        "page did not surface the stale-version stop: {}",
        followed.body
    );

    let after_record = std::fs::read_to_string(&record_path).unwrap();
    let after_ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert_eq!(
        before_record, after_record,
        "the page must write nothing to the topic file on a stale version"
    );
    assert_eq!(
        before_ledger, after_ledger,
        "the page must write nothing to the ledger on a stale version"
    );

    let _ = child.kill();
    let _ = child.wait();
}

// adr: 11be073c-ce91-44d8-833a-ef0b5e0b1672
//
// The page runs the same letter parsing the CLI does (`actions::parse_letter`),
// so a lowercase `answer=a` refuses with `invalid_letter`, the redirect
// itself carrying the message in its `m` query param -- and, followed, the
// page shows it. Nothing is written to the topic file or the ledger.
#[test]
fn post_answer_with_an_invalid_letter_redirects_with_invalid_letter_text() {
    let dir = setup_repo_with_open_question();
    let record_path = dir.join("adrdom/decisions/example.md");
    let content = std::fs::read_to_string(&record_path).unwrap();
    let ids = extract_ids(&content);
    let question_id = ids[0].clone();

    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let cookie = format!("adr_t={}", token);

    let before_record = std::fs::read_to_string(&record_path).unwrap();
    let before_ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();

    let version = current_version(&dir, &question_id);
    let form = format!("id={}&v={}&answer=a&t={}", question_id, version, token);
    let resp = post(port, "/answer", Some(&cookie), &form);
    assert_eq!(resp.status, 302, "body: {}", resp.body);
    let location = header(&resp, "Location").expect("Location header");
    assert!(
        location.contains("invalid%20letter%20a"),
        "redirect location does not carry the invalid_letter text: {}",
        location
    );

    let followed = get(port, location, Some(&cookie));
    assert_eq!(followed.status, 200, "body: {}", followed.body);
    assert!(
        followed.body.contains("invalid letter a"),
        "page did not surface the invalid_letter stop: {}",
        followed.body
    );

    let after_record = std::fs::read_to_string(&record_path).unwrap();
    let after_ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert_eq!(
        before_record, after_record,
        "the page must write nothing to the topic file on an invalid letter"
    );
    assert_eq!(
        before_ledger, after_ledger,
        "the page must write nothing to the ledger on an invalid letter"
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

const WORDS_RECORD: &str = r#"# Example

Context prose.

### 1.1 What counts as a decision?
depends: decision

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
    // "decisions" also never shows here, but only because "decision" is
    // itself a shipped common word (assets/common_words.txt) whose "s"
    // strips clean -- see get_next_no_entry_list_shows_an_unaliased_plural
    // for the actual term-plural rule.
    assert!(
        !no_entry_line.contains("decisions"),
        "no-term line: {}",
        no_entry_line
    );

    let _ = child.kill();
    let _ = child.wait();
}

const PLURAL_TERMS_MD: &str = "# Terms\n\n\
## widget\n\
id: 22222222-2222-2222-2222-222222222222\n\
definition: A fixture term with no alias.\n\
aliases: \n";

const PLURAL_RECORD: &str = r#"# Example

Context prose.

### 1.1 What counts as a widget?
depends: widget

Problem: the count of widgets is unclear.

Standard: widgets must be counted plainly.

- A) A widget is one unit.
  Why: reasons.
  Costs: costs.
- B) A widget is a batch.
  Why: other reasons.
  Costs: other costs.
"#;

#[test]
fn get_next_no_entry_list_shows_an_unaliased_plural() {
    // adr: 521e6211-344a-4b56-a1aa-07be956b3b29 -- a word pairs to a term
    // only by its name or an explicit alias (src/terms.rs); the "No term"
    // list (src/view.rs no_entry_words) follows the same rule, so an
    // un-aliased plural of a term name ("widgets" next to the term
    // "widget", whose alias list is empty) still shows up as no-term.
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let record = write_record(&dir, "adrdom/decisions/example.md", PLURAL_RECORD);
    let terms = write_record(&dir, "adrdom/terms.md", PLURAL_TERMS_MD);
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

    // The singular pairs to the term and shows under "Terms in this question".
    assert!(resp.body.contains("<dt>widget"), "body: {}", resp.body);

    let no_entry_start = resp.body.find("No term:").expect("No term line present");
    let no_entry_line = &resp.body[no_entry_start..];
    let no_entry_line = &no_entry_line[..no_entry_line.find("</p>").unwrap_or(no_entry_line.len())];
    assert!(
        no_entry_line.contains("widgets"),
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

const UUID_INLINE_RECORD: &str = r#"# Example

Context prose.

### 1.1 What counts as a decision?
depends: decision

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
    assert!(adr_answer(&dir, &question_id, "A")
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

// Per "Does the tool stop while the terms file waits for approval?"
// (aa475aa3 A): the page shows the file's own current text plus a "waiting
// for approval" note and the approve button, never a before/after diff --
// there is no more approved snapshot on disk for it to diff against.
#[test]
fn terms_article_shows_waiting_for_approval_when_unapproved() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let terms = write_record(&dir, "adrdom/terms.md", WORDS_TERMS_MD);
    assert!(adr_owner(&dir, &["approve", terms.to_str().unwrap()], "")
        .status
        .success());
    // AGENTS.md and the hooks staged too: the pre-commit hook judges the
    // staged tree (per "Does check at a commit read the staged files?"),
    // and check holds every file install wrote, which sit on disk,
    // unstaged, since install.
    run_git(
        &dir,
        &[
            "add",
            "adrdom/terms.md",
            "adrdom/state/ledger.tsv",
            "adrdom/state/reader.tsv",
            "adrdom/.gitignore",
            "adrdom/state/terms-brief.md",
            "adrdom/.gitattributes",
            "AGENTS.md",
            ".githooks",
        ],
    );
    let commit_out = run_git(&dir, &["commit", "-q", "-m", "approve terms v1"]);
    assert!(commit_out.status.success(), "{:?}", commit_out);

    // Edit the definition without re-approving.
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
        !resp.body.contains("class=\"changed-box\""),
        "changed-box should be gone: {}",
        resp.body
    );
    assert!(
        resp.body.contains("waiting for approval"),
        "waiting note missing: {}",
        resp.body
    );
    assert!(
        resp.body.contains("Approve the terms"),
        "approve button missing: {}",
        resp.body
    );
    // The page shows the file's own (unapproved, working-tree) text, not
    // the last approved text.
    assert!(
        !resp.body.contains("A choice answered by the owner."),
        "should not show the old approved text: {}",
        resp.body
    );
    assert!(
        resp.body
            .contains("A choice the owner answers in, once, in writing."),
        "current text missing: {}",
        resp.body
    );

    let _ = child.kill();
    let _ = child.wait();
}

// adr: df452a5a-1e2d-497e-93e9-7d6a80e8540d
//
// "`adr serve` is an owner command (refuses without an interactive
// terminal)": no `ADR_OWNER_BYPASS` here, and stdin is `Stdio::null()`
// rather than a TTY, so the process must exit refusing before ever
// printing an `open:` line or binding a port.
#[test]
fn serve_refuses_without_an_interactive_terminal() {
    // 9.2 A: the owner gate is stdin being a TTY, not an env var. A
    // non-interactive stdin (Stdio::null() here) refuses regardless.
    let dir = setup_repo_with_open_question();
    let out = Command::new(adr_bin())
        .current_dir(&dir)
        .args(["serve", "--port", "0"])
        .env_remove("CLAUDECODE")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("adr serve should run");
    assert_eq!(out.status.code(), Some(2), "stderr: {}", stderr(&out));
    assert!(
        stderr(&out).contains("this is the owner's command"),
        "stderr: {}",
        stderr(&out)
    );
    assert!(
        stderr(&out).contains("owner runs: adr serve"),
        "stderr: {}",
        stderr(&out)
    );
}

// --- tiers, "stands on", refuse, refused, moot ----------------------------

#[test]
fn get_next_shows_tier_and_stands_on_and_a_refuse_button() {
    let dir = setup_repo_with_open_question();
    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let cookie = format!("adr_t={}", token);

    let resp = get(port, "/next", Some(&cookie));
    assert_eq!(resp.status, 200, "body: {}", resp.body);

    // "<topic>, tier N. Question p of n open." — EXAMPLE_RECORD's only
    // question has no heading dep, so it is tier 1.
    assert!(resp.body.contains("Example, tier 1"), "body: {}", resp.body);
    assert!(
        resp.body.contains("Question 1 of 1 open"),
        "body: {}",
        resp.body
    );

    // "Stands on:" lists the entries as written: EXAMPLE_RECORD's question
    // carries `depends: example` (a term entry).
    assert!(
        resp.body.contains("Stands on: example"),
        "body: {}",
        resp.body
    );

    // A Refuse button beside the Answer button, posting `answer=refuse` to
    // the same route.
    assert!(
        resp.body
            .contains("name=\"answer\" value=\"refuse\" class=\"refuse\""),
        "body: {}",
        resp.body
    );
    assert!(resp.body.contains(">Refuse<"), "body: {}", resp.body);

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn overview_shows_tier_n_is_open() {
    let dir = setup_repo_with_open_question();
    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let cookie = format!("adr_t={}", token);

    let resp = get(port, "/", Some(&cookie));
    assert_eq!(resp.status, 200, "body: {}", resp.body);
    assert!(resp.body.contains("Tier 1 is open"), "body: {}", resp.body);

    let _ = child.kill();
    let _ = child.wait();
}

const REFUSED_RECORD: &str = "# Refused Topic\n\n\
### 1.1 Root?\n\n\
id: 11111111-1111-1111-1111-111111111111\n\n\
Problem: p.\n\n\
Standard: s.\n\n\
- A) x.\n  Why: w.\n  Costs: c.\n\
- B) y.\n  Why: w.\n  Costs: c.\n\n\
**Answer:** refused\n";

/// The refused state is set by hand — a ledger row plus the `**Answer:**
/// refused` line — rather than through `adr answer <id> refuse`, which is
/// being built on this same branch by another agent (see the wave-2
/// report).
#[test]
fn overview_lists_a_refused_question_with_the_label() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    write_record(&dir, "adrdom/decisions/refused.md", REFUSED_RECORD);
    append_ledger_row(
        &dir,
        "asked",
        "11111111-1111-1111-1111-111111111111",
        "h",
        "1.1 Root?",
    );
    append_ledger_row(
        &dir,
        "refused",
        "11111111-1111-1111-1111-111111111111",
        "h",
        "1.1 Root?",
    );

    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let cookie = format!("adr_t={}", token);

    let resp = get(port, "/", Some(&cookie));
    assert_eq!(resp.status, 200, "body: {}", resp.body);
    assert!(
        resp.body.contains("<span class=\"label refused\">refused</span>"),
        "body: {}",
        resp.body
    );
    assert!(resp.body.contains("1.1 Root?"), "body: {}", resp.body);

    let _ = child.kill();
    let _ = child.wait();
}

// --- owner.md: does the adr binary serve the ui / what the ui requires ---

/// Two independent (no depends between them) tier-1 questions, both
/// standing on the "example" term, so one repo can carry a question the
/// CLI answers and a separate one the page answers.
const TWO_QUESTION_RECORD: &str = r#"# Example

### 1.1 What is the unit of authority called?
depends: example

Problem: names leaked into decisions.

Standard: plain words.

- A) The authoritative sentence is a decision, and the question that produced it is a question.
  Why: plainest word.
  Costs: two nouns.
- B) The whole unit is a block.
  Why: matches predecessors.
  Costs: names a region.

### 1.2 What is the second unit of authority called?
depends: example

Problem: naming the second unit, also a decision.

Standard: plain words.

- A) The second thing is a widget, and a decision besides.
  Why: reasons.
  Costs: costs.
- B) The second thing is a gadget.
  Why: other reasons.
  Costs: other costs.
"#;

// adr: ece60954-56d4-4622-883e-265f3b997c9f
//
// "every act on it calls the same code as the command": answering one
// question through `adr answer` and an independent one through the page's
// `POST /answer` writes the identical shape of file line
// (`**Answer:** <letter>`) and ledger row (`answered`, the question's own
// asked-hash, the letter, the running binary's version) either way -- both
// go through `actions::do_answer`. And "a stop the command gives, the page
// gives too": re-answering an already-answered id refuses with the same
// "is already answered" text from `msg::already_answered` whichever side
// asks.
#[test]
fn page_answer_and_cli_answer_write_the_same_shaped_file_line_and_ledger_row_and_share_stops() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["decision", "example"]);
    let record = write_record(&dir, "adrdom/decisions/example.md", TWO_QUESTION_RECORD);
    approve_record(&dir, "adrdom/decisions/example.md");
    let ask_out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(ask_out.status.success(), "ask failed: {}", stderr(&ask_out));

    let content = std::fs::read_to_string(&record).unwrap();
    let ids = extract_ids(&content);
    assert_eq!(ids.len(), 2, "fixture should carry two ids: {:?}", ids);
    let (cli_id, page_id) = (ids[0].clone(), ids[1].clone());

    let ledger_before = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    let asked_hash = |id: &str| -> String {
        ledger_before
            .lines()
            .filter(|l| l.contains("\tasked\t") && l.contains(id))
            .last()
            .unwrap_or_else(|| panic!("no asked row for {}: {}", id, ledger_before))
            .split('\t')
            .nth(3)
            .unwrap()
            .to_string()
    };
    let cli_hash = asked_hash(&cli_id);
    let page_hash = asked_hash(&page_id);

    // The CLI path: `adr answer <id> A`.
    let answer_out = adr_answer(&dir, &cli_id, "A");
    assert!(
        answer_out.status.success(),
        "cli answer failed: {}",
        stderr(&answer_out)
    );

    // The page path: POST /answer with the cookie and the matching token.
    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let cookie = format!("adr_t={}", token);
    // Per "Does an answer carry the version of the question the owner
    // read?": the page's POST carries the version, the same 8 hex
    // characters computed from `page_hash` above.
    let form = format!("id={}&v={}&answer=A&t={}", page_id, &page_hash[..8], token);
    let resp = post(port, "/answer", Some(&cookie), &form);
    assert_eq!(resp.status, 302, "body: {}", resp.body);

    // Same file line: both sections get the exact same "**Answer:** A"
    // text, regardless of which caller reached `actions::do_answer`.
    let content_after = std::fs::read_to_string(&record).unwrap();
    let answer_line_count = content_after
        .lines()
        .filter(|l| l.trim() == "**Answer:** A")
        .count();
    assert_eq!(
        answer_line_count, 2,
        "expected the CLI's and the page's answer to both write the identical \"**Answer:** A\" line: {}",
        content_after
    );

    // Same ledger row shape: 6 tab-separated fields, event=answered, the
    // question's own asked-hash, detail=the letter, and the same version
    // field (one running binary), for both the CLI row and the page row.
    let ledger_after = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    let answered_row = |id: &str| -> Vec<String> {
        ledger_after
            .lines()
            .filter(|l| l.contains("\tanswered\t") && l.contains(id))
            .last()
            .unwrap_or_else(|| panic!("no answered row for {}: {}", id, ledger_after))
            .split('\t')
            .map(|s| s.to_string())
            .collect()
    };
    let cli_row = answered_row(&cli_id);
    let page_row = answered_row(&page_id);
    assert_eq!(cli_row.len(), 6, "{:?}", cli_row);
    assert_eq!(page_row.len(), 6, "{:?}", page_row);
    assert_eq!(cli_row[1], "answered");
    assert_eq!(page_row[1], "answered");
    assert_eq!(cli_row[3], cli_hash, "cli row hash should be its own asked-hash");
    assert_eq!(page_row[3], page_hash, "page row hash should be its own asked-hash");
    assert_eq!(cli_row[4], "A");
    assert_eq!(page_row[4], "A");
    assert_eq!(
        cli_row[5], page_row[5],
        "same binary answering both, so the same version field"
    );

    // Same stop, both ways: re-answering an already-answered id refuses,
    // with the identical "is already answered" text, whichever side asks.
    let cli_stop = adr_answer(&dir, &cli_id, "B");
    assert!(!cli_stop.status.success());
    let cli_stop_err = stderr(&cli_stop);
    assert!(
        cli_stop_err.contains("is already answered"),
        "{}",
        cli_stop_err
    );

    let form2 = format!("id={}&answer=B&t={}", page_id, token);
    let resp2 = post(port, "/answer", Some(&cookie), &form2);
    assert_eq!(resp2.status, 302, "body: {}", resp2.body);
    let location2 = header(&resp2, "Location").expect("Location header");
    let followed = get(port, location2, Some(&cookie));
    assert_eq!(followed.status, 200, "body: {}", followed.body);
    assert!(
        followed.body.contains("is already answered"),
        "page did not surface the same stop: {}",
        followed.body
    );

    let _ = child.kill();
    let _ = child.wait();
}

// --- owner.md: what the ui requires before an owner act -------------------

// adr: df452a5a-1e2d-497e-93e9-7d6a80e8540d
//
// "prints a fresh token once per start": two separate starts of `adr
// serve` on the same repo print two different tokens.
#[test]
fn serve_prints_a_fresh_token_each_start() {
    let dir = setup_repo_with_open_question();

    let mut child1 = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (_port1, token1) = read_open_line(&mut child1);
    let _ = child1.kill();
    let _ = child1.wait();

    let mut child2 = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (_port2, token2) = read_open_line(&mut child2);
    let _ = child2.kill();
    let _ = child2.wait();

    assert_ne!(
        token1, token2,
        "each start of adr serve must print a fresh token"
    );
}

// adr: df452a5a-1e2d-497e-93e9-7d6a80e8540d
//
// "listens on 127.0.0.1 only" (the `open:` line `read_open_line` parses
// here is the printed `http://127.0.0.1:<port>`, asserted by its own
// prefix check) "and every act verifies the cookie (an act without the
// cookie or with a wrong one is refused and writes nothing)": a POST with
// no `Cookie` header, and one with a wrong `adr_t` value, both refuse and
// leave the topic file and the ledger byte-for-byte unchanged.
#[test]
fn serve_binds_loopback_and_every_act_verifies_the_cookie_or_writes_nothing() {
    let dir = setup_repo_with_open_question();
    let record_path = dir.join("adrdom/decisions/example.md");
    let before_record = std::fs::read_to_string(&record_path).unwrap();
    let before_ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();

    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child); // asserts the 127.0.0.1 prefix

    let content = std::fs::read_to_string(&record_path).unwrap();
    let question_id = extract_ids(&content)[0].clone();
    let form = format!("id={}&answer=A&t={}", question_id, token);

    // No cookie at all, even with the correct token in the form.
    let resp = post(port, "/answer", None, &form);
    assert_eq!(resp.status, 403, "body: {}", resp.body);

    // A wrong cookie value.
    let resp2 = post(port, "/answer", Some("adr_t=not-the-token"), &form);
    assert_eq!(resp2.status, 403, "body: {}", resp2.body);

    let after_record = std::fs::read_to_string(&record_path).unwrap();
    let after_ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert_eq!(
        before_record, after_record,
        "an unverified post must not touch the topic file"
    );
    assert_eq!(
        before_ledger, after_ledger,
        "an unverified post must not touch the ledger"
    );

    let _ = child.kill();
    let _ = child.wait();
}

// adr: 0a742360-83c2-479d-b858-4bea646c9b36
//
// "the page hold[s] none" / "the page's question view renders no lean":
// the fixture question below carries options, a Problem and a Standard
// like any other, and `/next`'s rendering of it has no "Lean" text
// anywhere -- the page template has no field for one to land in. The
// other half of this decision (the topic file rejects a `Lean:` line) is
// tied in decision_test.rs's `a_lean_line_in_the_topic_file_is_stray`.
#[test]
fn question_page_never_renders_a_lean_field() {
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
    assert!(
        !resp.body.to_lowercase().contains("lean"),
        "the question page must never render a lean: {}",
        resp.body
    );

    let _ = child.kill();
    let _ = child.wait();
}

// adr: 644b43ce-be10-4e9a-a7d5-71b27c51e365
//
// "approve ... run[s] from the page": posting to `/approve` with the
// owner's cookie and token drives the same `actions::do_approve` a real
// `adr approve` runs -- it mints the terms row, so the terms file is
// approved afterward with no terminal `adr approve` call at all.
#[test]
fn post_approve_with_cookie_and_token_approves_terms_from_the_page() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    write_record(&dir, "adrdom/terms.md", WORDS_TERMS_MD);

    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let cookie = format!("adr_t={}", token);

    let form = format!("file=adrdom/terms.md&t={}", token);
    let resp = post(port, "/approve", Some(&cookie), &form);
    assert_eq!(resp.status, 302, "body: {}", resp.body);
    let location = header(&resp, "Location").expect("Location header");
    assert!(location.starts_with("/?m="), "{}", location);

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        ledger.lines().any(|l| l.contains("\tterms\t")),
        "ledger missing terms row after a page approve: {}",
        ledger
    );

    // The overview no longer offers an "Approve terms" button, and marks
    // the terms as approved -- proof the approve the page just ran is the
    // real thing, not a no-op.
    let overview = get(port, "/", Some(&cookie));
    assert_eq!(overview.status, 200, "body: {}", overview.body);
    assert!(
        overview.body.contains("approved"),
        "body: {}",
        overview.body
    );
    assert!(
        !overview.body.contains("Approve terms"),
        "terms should no longer need approving: {}",
        overview.body
    );

    let _ = child.kill();
    let _ = child.wait();
}

// adr: 644b43ce-be10-4e9a-a7d5-71b27c51e365
//
// "unban ... run[s] from the page": posting to `/unban` with the owner's
// cookie and token drives the same `actions::do_unban` a real
// `adr unban` runs -- the banned hash is actually removed and the ledger
// gets its `unbanned` row, with no terminal `adr unban` call at all.
#[test]
fn post_unban_with_cookie_and_token_removes_a_banned_phrase_from_the_page() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    let ban_out = adr(&dir, &["ban"], &[], "forbiddenword\n");
    assert!(ban_out.status.success(), "ban failed: {}", stderr(&ban_out));
    let banned_before =
        std::fs::read_to_string(dir.join("adrdom/state/banned.sha256")).unwrap();
    assert!(!banned_before.trim().is_empty());

    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let cookie = format!("adr_t={}", token);

    let form = format!("lines=forbiddenword&t={}", token);
    let resp = post(port, "/unban", Some(&cookie), &form);
    assert_eq!(resp.status, 302, "body: {}", resp.body);
    let location = header(&resp, "Location").expect("Location header");
    assert!(location.starts_with("/?m="), "{}", location);

    let banned_after =
        std::fs::read_to_string(dir.join("adrdom/state/banned.sha256")).unwrap();
    assert!(
        banned_after.trim().is_empty(),
        "expected the hash removed by a page unban: {}",
        banned_after
    );

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        ledger.lines().any(|l| l.contains("\tunbanned\t")),
        "ledger missing unbanned row after a page unban: {}",
        ledger
    );

    let _ = child.kill();
    let _ = child.wait();
}

// adr: 644b43ce-be10-4e9a-a7d5-71b27c51e365
//
// "the page shows every topic file's state, open questions with their
// options, current decisions and check results": exercises the pieces
// the tests above don't already cover -- an open question's own options
// (on `/next`), and the answered-decision list (on `/decisions`, unhit
// by any other test in this file). Topic-file state, and check's own
// report, are covered by `get_root_with_cookie_shows_fixture_topic`,
// `overview_shows_tier_n_is_open` and the moot/refused-state tests above.
#[test]
fn page_shows_question_options_and_the_decisions_list() {
    let dir = setup_repo_with_open_question();

    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let cookie = format!("adr_t={}", token);

    // The open question's own options, from EXAMPLE_RECORD, are on /next.
    // "decision" is a paired term here (approve_terms_with above), so
    // option A's sentence is split around the word the page wraps in a
    // term span rather than matched as one literal run.
    let next = get(port, "/next", Some(&cookie));
    assert_eq!(next.status, 200, "body: {}", next.body);
    assert!(
        next.body.contains("The authoritative sentence is a"),
        "option A's opening missing from the page: {}",
        next.body
    );
    assert!(
        next.body
            .contains(", and the question that produced it is a question."),
        "option A's close missing from the page: {}",
        next.body
    );
    assert!(
        next.body.contains("The whole unit is a block."),
        "option B missing from the page: {}",
        next.body
    );

    // Answer it for real, then /decisions shows it as a current decision
    // with the sentence the owner picked.
    let content = std::fs::read_to_string(dir.join("adrdom/decisions/example.md")).unwrap();
    let question_id = extract_ids(&content)[0].clone();
    assert!(adr_answer(&dir, &question_id, "A")
        .status
        .success());

    let decisions = get(port, "/decisions", Some(&cookie));
    assert_eq!(decisions.status, 200, "body: {}", decisions.body);
    assert!(
        decisions.body.contains("Current decisions"),
        "body: {}",
        decisions.body
    );
    assert!(
        decisions.body.contains("What is the unit of authority called?"),
        "answered heading missing from /decisions: {}",
        decisions.body
    );
    assert!(
        decisions.body.contains(
            "The authoritative sentence is a decision, and the question that produced it is a question."
        ),
        "chosen sentence missing from /decisions: {}",
        decisions.body
    );

    // adr: 7cad9922-9bd1-4c1a-aedc-df1f77c83e1f
    // A supersedes entry is written as an id; the page turns it back into
    // the heading it names, never printing the bare id.
    let superseding = format!(
        "# Second\n\n### 2.1 Is the decision revisited?\ndepends: decision\n\nsupersedes: {}\n\nProblem: the decision needs revisiting.\n\nStandard: s2.\n\n- A) Yes: the decision is revisited.\n  Why: reasons.\n  Costs: costs.\n- B) No: the decision stands.\n  Why: other reasons.\n  Costs: other costs.\n",
        question_id
    );
    let second = write_record(&dir, "adrdom/decisions/second.md", &superseding);
    approve_record(&dir, "adrdom/decisions/second.md");
    let ask_out = adr(&dir, &["ask", second.to_str().unwrap()], &[], "");
    assert!(ask_out.status.success(), "ask failed: {}", stderr(&ask_out));
    let second_content = std::fs::read_to_string(&second).unwrap();
    let second_id = extract_ids(&second_content)[0].clone();
    assert!(adr_answer(&dir, &second_id, "A").status.success());

    let decisions = get(port, "/decisions", Some(&cookie));
    assert_eq!(decisions.status, 200, "body: {}", decisions.body);
    assert!(
        decisions.body.contains("supersedes 1.1 What is the unit of authority called?"),
        "superseded heading missing from /decisions: {}",
        decisions.body
    );
    assert!(
        !decisions.body.contains(&question_id),
        "bare id printed on /decisions: {}",
        decisions.body
    );

    let _ = child.kill();
    let _ = child.wait();
}

// --- core.md: tiers derived from depends lines, and a reference resolved
// back into a name -- both on the page, not just on `show` -------------

/// Tier 1, no depends line at all (mirrors `ask_tiers_test.rs`'s Q1,
/// proven to pair the "widget" term through a real `ask`).
const TIER_Q1_RECORD: &str = r#"# Example

### 1.1 What color is the widget?
depends: widget

Problem: the widget's color is undecided.

Standard: plain words.

- A) The widget is red.
  Why: reasons.
  Costs: costs.
- B) The widget is blue.
  Why: other reasons.
  Costs: other costs.
"#;

/// Appended after Q1 is answered: never written with a tier of its own --
/// its `depends:` line just names Q1's heading text with a mandatory
/// set -- so its tier (2) is entirely the tool's own derivation.
const TIER_Q2_APPENDIX: &str = "\n### 1.2 Does the widget's paint match its color?\n\ndepends: 1.1 What color is the widget?\n\nProblem: the paint might not match.\n\nStandard: plain words.\n\n- A) Yes.\n  Why: reasons.\n  Costs: costs.\n- B) No.\n  Why: other reasons.\n  Costs: other costs.\n";

// adr: aec860fd-22ad-44a7-99f7-222ea19537e7
//
// "show and the page print the tiers": Q1 is tier 1 (no depends line at
// all); Q2, appended later and never written with a tier of its own,
// stands on Q1's heading and so is tier 2, purely derived. As the topic
// advances from Q1 open to Q2 open, the page's own tier line moves with
// it on both routes it appears on -- "<topic>, tier N" on /next and
// "Tier N is open" on the overview -- proof the page computes the tier
// itself rather than reading one written in the file.
#[test]
fn page_prints_the_tiers_as_the_topic_advances() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let record = write_record(&dir, "adrdom/decisions/example.md", TIER_Q1_RECORD);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let cookie = format!("adr_t={}", token);

    let next1 = get(port, "/next", Some(&cookie));
    assert_eq!(next1.status, 200, "body: {}", next1.body);
    assert!(next1.body.contains("Example, tier 1"), "body: {}", next1.body);
    let overview1 = get(port, "/", Some(&cookie));
    assert_eq!(overview1.status, 200, "body: {}", overview1.body);
    assert!(
        overview1.body.contains("Tier 1 is open"),
        "body: {}",
        overview1.body
    );

    let content = std::fs::read_to_string(&record).unwrap();
    let q1_id = extract_ids(&content)[0].clone();
    assert!(adr_answer(&dir, &q1_id, "A")
        .status
        .success());

    let mut content2 = std::fs::read_to_string(&record).unwrap();
    content2.push_str(TIER_Q2_APPENDIX);
    std::fs::write(&record, &content2).unwrap();
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    // Same routes, no restart: the page re-reads the file/ledger fresh on
    // every request, so it now reports tier 2 with nothing written in the
    // topic file naming a tier at all.
    let next2 = get(port, "/next", Some(&cookie));
    assert_eq!(next2.status, 200, "body: {}", next2.body);
    assert!(next2.body.contains("Example, tier 2"), "body: {}", next2.body);
    let overview2 = get(port, "/", Some(&cookie));
    assert_eq!(overview2.status, 200, "body: {}", overview2.body);
    assert!(
        overview2.body.contains("Tier 2 is open"),
        "body: {}",
        overview2.body
    );

    let _ = child.kill();
    let _ = child.wait();
}

// adr: 7cad9922-9bd1-4c1a-aedc-df1f77c83e1f
//
// "every reference carries the identifier of what it names; the agent
// writes the name, approve turns it into the identifier, and show and the
// page turn it back into the name": Q2's depends line is written by hand
// naming Q1's heading text; approve rewrites it on disk to Q1's freshly
// minted uuid (asserted below, so the id-vs-name distinction is real, not
// assumed). The page's "Stands on:" line for Q2 still prints the heading
// words with the set restored -- never the raw uuid the file
// actually carries.
#[test]
fn page_shows_a_heading_depends_entry_by_name_not_id() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let record = write_record(&dir, "adrdom/decisions/example.md", TIER_Q1_RECORD);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    let content = std::fs::read_to_string(&record).unwrap();
    let q1_id = extract_ids(&content)[0].clone();
    assert!(adr_answer(&dir, &q1_id, "A")
        .status
        .success());

    let mut content2 = std::fs::read_to_string(&record).unwrap();
    content2.push_str(TIER_Q2_APPENDIX);
    std::fs::write(&record, &content2).unwrap();
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "")
        .status
        .success());

    // On disk, Q2's depends line now carries Q1's raw uuid, not its name.
    let approved = std::fs::read_to_string(&record).unwrap();
    assert!(
        approved.contains(&format!("depends: {}", q1_id)),
        "expected the depends line rewritten to the id on disk: {}",
        approved
    );

    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let cookie = format!("adr_t={}", token);

    let next = get(port, "/next", Some(&cookie));
    assert_eq!(next.status, 200, "body: {}", next.body);
    assert!(
        next.body
            .contains("Stands on: 1.1 What color is the widget?"),
        "expected the page to turn the id back into the heading name: {}",
        next.body
    );
    assert!(
        !next.body.contains(&q1_id),
        "the page must never leak the raw id: {}",
        next.body
    );

    let _ = child.kill();
    let _ = child.wait();
}

// adr: 644b43ce-be10-4e9a-a7d5-71b27c51e365
//
// "answer ... run[s] from the page": POSTing to /answer with the owner's
// cookie and token writes both of the two things a real `adr answer`
// writes -- the topic file's own "**Answer:** A" line and the ledger's
// "answered" row -- with no terminal `adr answer` call at all.
#[test]
fn post_answer_through_the_page_writes_the_answer_to_the_file_and_ledger() {
    let dir = setup_repo_with_open_question();
    let content = std::fs::read_to_string(dir.join("adrdom/decisions/example.md")).unwrap();
    let question_id = extract_ids(&content)[0].clone();

    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let cookie = format!("adr_t={}", token);

    // Per "Does an answer carry the version of the question the owner
    // read?": the page's POST carries the version shown on the form it
    // displayed.
    let version = current_version(&dir, &question_id);
    let form = format!("id={}&v={}&answer=A&t={}", question_id, version, token);
    let resp = post(port, "/answer", Some(&cookie), &form);
    assert_eq!(resp.status, 302, "body: {}", resp.body);

    let record_after = std::fs::read_to_string(dir.join("adrdom/decisions/example.md")).unwrap();
    assert!(
        record_after.lines().any(|l| l.trim() == "**Answer:** A"),
        "expected the page's answer written to the topic file: {}",
        record_after
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

// adr: da59b448-1afb-4469-b40e-69eda8f253c2
//
// The page must show the same approval state the CLI does. B's `depends:`
// line, hand-reverted back to the heading words `approve` rewrote to A's
// id, still resolves to that same id in memory (the same cross-file
// pre-pass `snapshot` runs before reading approval state) -- but it is no
// longer the bytes the owner approved, exactly the fixture
// `ask_and_check_refuse_a_cross_file_depends_line_reverted_by_hand_after_approve`
// proves `ask`/`check` catch. The page must catch it too: `snapshot` reads
// plan/context approval off each file before its own normalize pass
// mutates the parsed `depends:` entries in memory, and hands those values
// straight into `build_file_view` as parameters (rather than letting it
// recompute its own, wrong, copy off the by-then-normalized file) -- so
// there is no window where the page could show B as approved.
#[test]
fn page_shows_a_reverted_cross_file_depends_line_as_not_approved() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "decision");

    // A: a real, fully approved current decision of its own.
    let a_content = "# Topic A\n\n### 1.1 Is A do a thing?\ndepends: decision\n\nProblem: a decision.\n\nStandard: a decision.\n\n- A) x, a decision.\n  Why: w.\n  Costs: c.\n- B) y, a decision.\n  Why: w.\n  Costs: c.\n";
    let record_a = write_record(&dir, "adrdom/decisions/topic-a.md", a_content);
    let out_a = adr_owner(&dir, &["approve", record_a.to_str().unwrap()], "");
    assert!(out_a.status.success(), "approving A failed: {}", stderr(&out_a));
    let approved_a = std::fs::read_to_string(&record_a).unwrap();
    let a_id = approved_a
        .lines()
        .find_map(|l| l.strip_prefix("id: "))
        .unwrap()
        .to_string();
    let ask_a = adr(&dir, &["ask", record_a.to_str().unwrap()], &[], "");
    assert!(ask_a.status.success(), "asking A failed: {}", stderr(&ask_a));
    assert!(
        adr_answer(&dir, &a_id, "A").status.success(),
        "answering A failed"
    );

    // B: its only question stands on A, named by A's heading text.
    let b_text = "1.1 Is A do a thing?";
    let b_content = format!(
        "# Topic B\n\n### 1.1 Is B stand on A?\ndepends: {}\n\nProblem: p2.\n\nStandard: s2.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        b_text
    );
    let record_b = write_record(&dir, "adrdom/decisions/topic-b.md", &b_content);
    let out_b = adr_owner(&dir, &["approve", record_b.to_str().unwrap()], "");
    assert!(out_b.status.success(), "approving B failed: {}", stderr(&out_b));

    let approved_b = std::fs::read_to_string(&record_b).unwrap();
    assert!(
        approved_b.contains(&format!("depends: {}\n", a_id)),
        "expected B's depends line rewritten to A's id: {}",
        approved_b
    );

    // Hand-revert the rewritten line back to the heading words `approve`
    // replaced it with.
    let reverted = approved_b.replacen(
        &format!("depends: {}\n", a_id),
        &format!("depends: {}\n", b_text),
        1,
    );
    assert_ne!(reverted, approved_b, "the fixture should still hold the rewritten line");
    std::fs::write(&record_b, &reverted).unwrap();

    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let cookie = format!("adr_t={}", token);

    let resp = get(port, "/", Some(&cookie));
    assert_eq!(resp.status, 200, "body: {}", resp.body);

    let b_start = resp.body.find("Topic B").expect("topic B on the page");
    let b_end = resp.body[b_start..]
        .find("</article>")
        .map(|i| b_start + i)
        .unwrap_or(resp.body.len());
    let b_block = &resp.body[b_start..b_end];
    assert!(
        b_block.contains("not approved"),
        "a hand-reverted depends line must still show as not approved on the page: {}",
        b_block
    );

    let a_start = resp.body.find("Topic A").expect("topic A on the page");
    let a_end = resp.body[a_start..]
        .find("</article>")
        .map(|i| a_start + i)
        .unwrap_or(resp.body.len());
    let a_block = &resp.body[a_start..a_end];
    assert!(
        !a_block.contains("not approved"),
        "A is genuinely approved and must not show as waiting: {}",
        a_block
    );

    let _ = child.kill();
    let _ = child.wait();
}
