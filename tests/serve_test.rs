mod common;

use common::*;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

/// A running `adr serve`, killed when it goes out of scope, so a test that
/// fails before its own kill leaves no server behind.
struct Served(Child);

impl std::ops::Deref for Served {
    type Target = Child;
    fn deref(&self) -> &Child {
        &self.0
    }
}

impl std::ops::DerefMut for Served {
    fn deref_mut(&mut self) -> &mut Child {
        &mut self.0
    }
}

impl Drop for Served {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Mirrors `common::adr_owner`'s env, but spawns the process (rather than
/// running it to completion) since `adr serve` never exits on its own; the
/// caller reads its `open:` line off stdout.
fn spawn_owner(dir: &Path, args: &[&str]) -> Served {
    Served(
        scrubbed(Command::new(adr_bin()))
            .current_dir(dir)
            .args(args)
            .env_remove("CLAUDECODE")
            .env("ADR_OWNER_BYPASS", "1")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("adr serve should spawn"),
    )
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

/// Follows the redirect the page answered an act with: the page it leads
/// to, the act's lines in its banner.
fn follow(port: u16, resp: &HttpResponse, cookie: &str) -> HttpResponse {
    let location = header(resp, "Location").expect("Location header");
    get(port, location, Some(cookie))
}

fn unhtml(s: &str) -> String {
    let mut text = String::new();
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            c if !in_tag => text.push(c),
            _ => {}
        }
    }
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

/// The lines `adr <args>` prints on standard output, each without its
/// indent.
fn cli_lines(dir: &Path, args: &[&str]) -> Vec<String> {
    stdout(&adr(dir, args, &[], ""))
        .lines()
        .map(|l| l.trim_start().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

/// A page of one file shows only lines the command prints for it, and every
/// status, finding and move the command prints is on the page: the owner's
/// moves as their forms and links, the others as text.
fn assert_page_shows(body: &str, shown: &[String]) {
    let page = page_lines(body);
    let words: Vec<String> = shown.iter().map(|l| page_words(l)).collect();
    for l in &page {
        assert!(words.contains(l), "the page shows {:?}, which the command does not print: {:#?}", l, shown);
    }
    for l in shown {
        let bare = l.strip_prefix("+ ").or_else(|| l.strip_prefix("- ")).unwrap_or(l);
        if is_owner_move(l) {
            assert_owner_move_shown(l, body);
        } else if bare.starts_with('[') || l.starts_with("agent ") || l.starts_with("not yet:") {
            assert!(page.contains(&page_words(l)), "the page lacks {:?}: {:#?}", l, page);
        }
    }
}

/// Every line `lines` holds is on the page: as text, except a topic's
/// heading line (the card's title) and the owner's moves (forms and links).
fn assert_on_page(lines: &[String], body: &str) {
    let page = page_lines(body);
    for l in lines {
        if is_owner_move(l) {
            assert_owner_move_shown(l, body);
        } else if !l.starts_with("# ") {
            assert!(page.contains(&page_words(l)), "the page lacks the line {:?}: {:#?}", l, page);
        }
    }
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
    assert!(resp.body.contains("<a href=\"/terms\">Terms</a>"), "body: {}", resp.body);

    let _ = child.kill();
    let _ = child.wait();
}

// adr: 11be073c-ce91-44d8-833a-ef0b5e0b1672
//
// "answer takes that version from the terminal and the page alike": the
// page's POST with the current version -- the hidden `v` field of the form
// under the owner's answer move -- answers, same as the terminal, and the
// page it leads to shows the event the terminal prints.
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
    let next = get(port, "/next", Some(&cookie));
    assert!(next.body.contains(&format!("name=\"v\" value=\"{}\"", version)), "body: {}", next.body);

    let form = format!("id={}&v={}&answer=A&t={}", question_id, version, token);
    let resp = post(port, "/answer", Some(&cookie), &form);
    assert_eq!(resp.status, 302, "body: {}", resp.body);
    // The redirect carries the key of the act's lines: shown once, on the
    // page it leads to.
    assert!(header(&resp, "Location").is_some_and(|l| l.starts_with("/next?r=")), "{:?}", header(&resp, "Location"));

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        ledger
            .lines()
            .any(|l| l.contains("\tanswered\t") && l.contains(question_id.as_str())),
        "ledger missing answered row: {}",
        ledger
    );
    let followed = follow(port, &resp, &cookie);
    assert!(
        page_lines(&followed.body)
            .iter()
            .any(|l| l.starts_with("answered: \"1.1 What is the unit of authority called?\" A: ")),
        "body: {}",
        followed.body
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
// so a lowercase `answer=a` refuses with `invalid_letter`, and the page the
// act leads to shows the stop as the terminal prints it. Nothing is written
// to the topic file or the ledger.
#[test]
fn post_answer_with_an_invalid_letter_shows_the_invalid_letter_stop() {
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

    let followed = follow(port, &resp, &cookie);
    assert_eq!(followed.status, 200, "body: {}", followed.body);
    let cli = adr(
        &dir,
        &["answer", &format!("{}@{}", question_id, version), "a"],
        &[("ADR_OWNER_BYPASS", "1")],
        "",
    );
    let stop: Vec<String> = stderr(&cli).lines().map(str::to_string).collect();
    assert!(stop.iter().any(|l| l.contains("invalid letter a")), "{:?}", stop);
    assert_on_page(&stop, &followed.body);

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

// adr: bceceda8-133a-4c9b-b439-b5c1a4414a02
//
// Wherever the page offers the owner's approve, what the approve signs
// stands above the form: on `/`, a file with an asked question and a
// question added since its approval shows the new question, marked, above
// its Approve button.
#[test]
fn overview_shows_the_change_an_approve_form_signs() {
    let dir = setup_repo_with_open_question();
    let path = dir.join("adrdom/decisions/example.md");
    let mut text = std::fs::read_to_string(&path).unwrap();
    text.push_str("\n### 1.9 Is the example kept?\ndepends: example\n\nProblem: the example decision is open.\n\nStandard: decisions are plain.\n\n- A) Yes.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n");
    std::fs::write(&path, text).unwrap();
    plan_record(&dir, "adrdom/decisions/example.md");
    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let resp = get(port, "/", Some(&format!("adr_t={}", token)));
    let _ = child.kill();
    let _ = child.wait();
    assert_eq!(resp.status, 200, "body: {}", resp.body);
    let form = resp.body.find("action=\"/approve\"").unwrap_or_else(|| panic!("no approve form: {}", resp.body));
    let added = resp.body.find("1.9 Is the example kept?").unwrap_or_else(|| panic!("the change is not shown: {}", resp.body));
    assert!(added < form, "the change stands above the form: {}", resp.body);
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
fn terms_page_shows_what_show_prints_of_the_terms_file() {
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

    // /terms is what `adr show adrdom/terms.md` prints, line for line.
    let resp = get(port, "/terms", Some(&cookie));
    assert_eq!(resp.status, 200, "body: {}", resp.body);
    let shown = cli_lines(&dir, &["show", "adrdom/terms.md"]);
    assert!(shown.iter().any(|l| l.contains("A choice answered by the owner.")), "{:?}", shown);
    assert!(resp.body.contains("A choice answered by the owner."), "body: {}", resp.body);
    assert_page_shows(&resp.body, &shown);

    let _ = child.kill();
    let _ = child.wait();
}

const WORDS_TERMS_MD_V2: &str = "# Terms\n\n\
## decision\n\
id: 11111111-1111-1111-1111-111111111111\n\
definition: A choice the owner answers in, once, in writing.\n\
aliases: \n";

// Per "Does show print what a file holds beyond its latest approval?"
// (bceceda8 A) and "Does the ui show what the terminal prints?" (7857d633
// A): a terms edit not yet approved shows on the page as `adr show
// adrdom/terms.md` prints it -- the file changed, the entry's change marked
// -- with the owner's approve move and the form that makes it.
#[test]
fn terms_page_shows_an_unapproved_edit_marked_with_the_approve_form() {
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

    // The overview names the terms file by its status.
    let overview = get(port, "/", Some(&cookie));
    assert_eq!(overview.status, 200, "body: {}", overview.body);
    assert!(
        page_lines(&overview.body).contains(&"[changed] adrdom/terms.md".to_string()),
        "body: {}",
        overview.body
    );

    // /terms is `adr show adrdom/terms.md`, the change marked, then the
    // owner's approve move with its form.
    let resp = get(port, "/terms", Some(&cookie));
    assert_eq!(resp.status, 200, "body: {}", resp.body);
    let shown = cli_lines(&dir, &["show", "adrdom/terms.md"]);
    assert_page_shows(&resp.body, &shown);
    assert!(shown.contains(&"owner runs: adr approve adrdom/terms.md".to_string()), "{:?}", shown);
    assert!(
        shown.iter().any(|l| l.contains("A choice the owner answers in, once, in writing.")),
        "{:?}",
        shown
    );
    assert!(
        resp.body.contains("action=\"/approve\"")
            && resp.body.contains("name=\"file\" value=\"adrdom/terms.md\""),
        "approve form missing: {}",
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
    let out = scrubbed(Command::new(adr_bin()))
        .current_dir(&dir)
        .args(["serve", "--port", "0"])
        .env_remove("CLAUDECODE")
        .env_remove("ADR_OWNER_BYPASS")
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
fn get_next_shows_the_lines_a_question_carries_and_a_refuse_button() {
    let dir = setup_repo_with_open_question();
    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let cookie = format!("adr_t={}", token);

    let resp = get(port, "/next", Some(&cookie));
    assert_eq!(resp.status, 200, "body: {}", resp.body);

    // The question as `adr show <file>` prints it: its status with its
    // version, and the depends line it carries (a term entry here).
    let lines = page_lines(&resp.body);
    assert!(
        lines
            .iter()
            .any(|l| l == "[asked] 1.1 What is the unit of authority called?"),
        "{:#?}",
        lines
    );
    assert!(resp.body.contains("<p class=\"stands-on\">Stands on: example</p>"), "body: {}", resp.body);

    // A Refuse button beside the Answer button, posting `answer=refuse` to
    // the same route.
    assert!(resp.body.contains("name=\"answer\" value=\"refuse\""), "body: {}", resp.body);
    assert!(resp.body.contains(">Refuse<"), "body: {}", resp.body);

    let _ = child.kill();
    let _ = child.wait();
}

#[test]
fn overview_shows_what_show_prints() {
    let dir = setup_repo_with_open_question();
    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let cookie = format!("adr_t={}", token);

    let resp = get(port, "/", Some(&cookie));
    assert_eq!(resp.status, 200, "body: {}", resp.body);
    let shown = cli_lines(&dir, &["show"]);
    assert!(shown.iter().any(|l| l.starts_with("owner runs: adr answer ")), "{:?}", shown);
    assert_on_page(&shown, &resp.body);

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

/// The refused state is set by hand: a ledger row plus the `**Answer:**
/// refused` line. The page shows the move `adr show` prints for it.
#[test]
fn overview_shows_the_move_of_a_refused_question() {
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
    let shown = cli_lines(&dir, &["show"]);
    assert!(shown.iter().any(|l| l.contains("\"1.1 Root?\" is refused")), "{:?}", shown);
    assert_on_page(&shown, &resp.body);

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
    assert!(header(&resp, "Location").is_some_and(|l| l.starts_with("/?r=")), "{:?}", header(&resp, "Location"));

    let ledger = std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap();
    assert!(
        ledger.lines().any(|l| l.contains("\tterms\t")),
        "ledger missing terms row after a page approve: {}",
        ledger
    );

    // The page the act leads to shows its event, and marks the terms as
    // approved with no approve form left for them -- proof the approve the
    // page just ran is the real thing, not a no-op.
    let overview = follow(port, &resp, &cookie);
    assert_eq!(overview.status, 200, "body: {}", overview.body);
    let lines = page_lines(&overview.body);
    assert!(lines.contains(&"approved: adrdom/terms.md".to_string()), "{:#?}", lines);
    assert!(lines.contains(&"[approved] adrdom/terms.md".to_string()), "{:#?}", lines);
    assert!(
        !overview.body.contains("name=\"file\" value=\"adrdom/terms.md\""),
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
    assert!(header(&resp, "Location").is_some_and(|l| l.starts_with("/?r=")), "{:?}", header(&resp, "Location"));
    let followed = follow(port, &resp, &cookie);
    assert!(
        page_lines(&followed.body).contains(&"unbanned: a word".to_string()),
        "body: {}",
        followed.body
    );

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
// options, current decisions and check results": every topic file's
// status line, an open question's own options (on `/next`), and the
// current decisions with the option picked (on `/`, as `adr show` prints
// them).
#[test]
fn page_shows_question_options_and_the_decisions_list() {
    let dir = setup_repo_with_open_question();

    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let cookie = format!("adr_t={}", token);

    let overview = get(port, "/", Some(&cookie));
    assert!(
        page_lines(&overview.body).contains(&"[approved] Example (adrdom/decisions/example.md)".to_string()),
        "body: {}",
        overview.body
    );

    // The open question's own options, from EXAMPLE_RECORD, are on /next.
    let next = get(port, "/next", Some(&cookie));
    assert_eq!(next.status, 200, "body: {}", next.body);
    assert!(
        next.body.contains("<span class=\"key\">A</span>")
            && unhtml(&next.body).contains("The authoritative sentence is a decision, and the question that produced it is a question."),
        "option A missing from the page: {}",
        next.body
    );
    assert!(
        next.body.contains("<span class=\"key\">B</span>") && next.body.contains("The whole unit is a block."),
        "option B missing from the page: {}",
        next.body
    );

    // Answer it for real, then / shows it as a current decision with the
    // sentence the owner picked.
    let content = std::fs::read_to_string(dir.join("adrdom/decisions/example.md")).unwrap();
    let question_id = extract_ids(&content)[0].clone();
    assert!(adr_answer(&dir, &question_id, "A")
        .status
        .success());

    let decisions = get(port, "/", Some(&cookie));
    assert_eq!(decisions.status, 200, "body: {}", decisions.body);
    let lines = page_lines(&decisions.body);
    assert!(lines.contains(&"1.1 What is the unit of authority called?".to_string()), "{:#?}", lines);
    // /decisions lists it with its status line and the option picked.
    let list = get(port, "/decisions", Some(&cookie));
    assert!(page_lines(&list.body).iter().any(|l| l.starts_with("[answered A] 1.1 ")), "{}", list.body);
    assert!(
        lines.iter().any(|l| l.contains(
            "The authoritative sentence is a decision, and the question that produced it is a question."
        )),
        "chosen sentence missing from /: {:#?}",
        lines
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

    let next = get(port, "/next", Some(&cookie));
    assert_eq!(next.status, 200, "body: {}", next.body);
    assert!(
        next.body.contains("<p class=\"stands-on\">Supersedes: 1.1 What is the unit of authority called?</p>"),
        "superseded heading missing from /next: {}",
        next.body
    );
    assert!(!next.body.contains(&question_id), "bare id printed on /next: {}", next.body);

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

// As the topic advances from Q1 asked to Q2 asked, the page re-reads the
// records on every request and shows what `adr show` prints at each step:
// the owner's answer move for the question asked, with its form, and the
// decision once it is answered.
#[test]
fn page_follows_the_topic_as_it_advances() {
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

    let content = std::fs::read_to_string(&record).unwrap();
    let q1_id = extract_ids(&content)[0].clone();
    let form_of = |id: &str| format!("name=\"id\" value=\"{}\"", id);

    let next1 = get(port, "/next", Some(&cookie));
    assert_eq!(next1.status, 200, "body: {}", next1.body);
    assert!(next1.body.contains(&form_of(&q1_id)), "body: {}", next1.body);
    let overview1 = get(port, "/", Some(&cookie));
    assert_eq!(overview1.status, 200, "body: {}", overview1.body);
    assert_on_page(&cli_lines(&dir, &["show"]), &overview1.body);
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

    // Same routes, no restart: Q2 is offered, Q1 is a decision.
    let q2_id = extract_ids(&std::fs::read_to_string(&record).unwrap())[1].clone();
    let next2 = get(port, "/next", Some(&cookie));
    assert_eq!(next2.status, 200, "body: {}", next2.body);
    assert!(next2.body.contains(&form_of(&q2_id)), "body: {}", next2.body);
    assert!(!next2.body.contains(&form_of(&q1_id)), "body: {}", next2.body);
    let overview2 = get(port, "/", Some(&cookie));
    assert_eq!(overview2.status, 200, "body: {}", overview2.body);
    let shown = cli_lines(&dir, &["show"]);
    assert!(shown.contains(&"1.1 What color is the widget?".to_string()), "{:?}", shown);
    assert_on_page(&shown, &overview2.body);

    let _ = child.kill();
    let _ = child.wait();
}

// adr: 7327cd58-ac58-40a4-8ef2-00072fb552b2
//
// The page offers a form only for a question `answer` takes: with Q1 and Q2
// (standing on Q1=A) both asked, /next offers Q1 and shows Q2 with the
// move that says what it waits for, a POST for Q2 is refused with the
// same text the terminal prints, and once Q1 is answered the page offers Q2.
#[test]
fn page_offers_only_the_questions_answer_takes() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let q2 = TIER_Q2_APPENDIX.replace("depends: 1.1 What color is the widget?", "depends: 1.1 What color is the widget?=A");
    let record = write_record(&dir, "adrdom/decisions/example.md", &format!("{}{}", TIER_Q1_RECORD, q2));
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "").status.success());
    let ids = extract_ids(&std::fs::read_to_string(&record).unwrap());
    let (q1_id, q2_id) = (ids[0].clone(), ids[1].clone());

    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let cookie = format!("adr_t={}", token);

    let form_of = |id: &str| format!("name=\"id\" value=\"{}\"", id);
    let first = get(port, "/next", Some(&cookie));
    assert!(first.body.contains(&form_of(&q1_id)), "body: {}", first.body);
    assert!(!first.body.contains(&form_of(&q2_id)), "body: {}", first.body);
    let lines = page_lines(&first.body);
    assert!(
        lines.iter().any(|l| l.starts_with("not yet: \"1.2 Does the widget's paint match its color?\" is asked and waits for")),
        "{:#?}",
        lines
    );

    let form = format!("id={}&v={}&answer=A&t={}", q2_id, current_version(&dir, &q2_id), token);
    let resp = post(port, "/answer", Some(&cookie), &form);
    assert_eq!(resp.status, 302, "body: {}", resp.body);
    let followed = follow(port, &resp, &cookie);
    assert!(followed.body.contains("has no decision yet"), "page did not show the refusal: {}", followed.body);

    assert!(adr_answer(&dir, &q1_id, "A").status.success());
    let next = get(port, "/next", Some(&cookie));
    assert!(next.body.contains(&form_of(&q2_id)), "body: {}", next.body);

    let _ = child.kill();
    let _ = child.wait();
}

// adr: 7327cd58-ac58-40a4-8ef2-00072fb552b2
//
// The page keeps to what `answer` takes. The question standing on another
// sits first in the file: the overview shows the moves `adr show` prints,
// the owner's answer for the later question and what the earlier one
// waits for; /next shows the earlier one with no form. After the later
// question is answered, /next offers the earlier one it unlocked.
#[test]
fn page_shows_a_waiting_question_without_a_form_and_offers_it_once_unlocked() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let q2 = TIER_Q2_APPENDIX.replace("depends: 1.1 What color is the widget?", "depends: 1.1 What color is the widget?=A");
    let record = write_record(&dir, "adrdom/decisions/example.md", &format!("# Example\n{}\n{}", q2, TIER_Q1_RECORD.trim_start_matches("# Example\n")));
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "").status.success());
    let ids = extract_ids(&std::fs::read_to_string(&record).unwrap());
    let (q2_id, q1_id) = (ids[0].clone(), ids[1].clone());

    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let cookie = format!("adr_t={}", token);

    let form_of = |id: &str| format!("name=\"id\" value=\"{}\"", id);
    let overview = get(port, "/", Some(&cookie));
    let shown = cli_lines(&dir, &["show"]);
    assert!(shown.iter().any(|l| l.contains("waits for")), "{:?}", shown);
    assert_on_page(&shown, &overview.body);
    // The overview links the owner's answer to /next, where the options
    // stand beside the form: no answer form on it.
    assert!(!overview.body.contains(&form_of(&q1_id)), "body: {}", overview.body);
    assert!(!overview.body.contains(&form_of(&q2_id)), "body: {}", overview.body);
    assert!(overview.body.contains("href=\"/next\""), "body: {}", overview.body);

    let first = get(port, "/next", Some(&cookie));
    assert!(first.body.contains(&form_of(&q1_id)), "body: {}", first.body);
    assert!(!first.body.contains(&form_of(&q2_id)), "body: {}", first.body);

    assert!(adr_answer(&dir, &q1_id, "A").status.success());
    let after_q1 = get(port, "/next", Some(&cookie));
    assert!(after_q1.body.contains(&form_of(&q2_id)), "body: {}", after_q1.body);

    let _ = child.kill();
    let _ = child.wait();
}

// adr: 7327cd58-ac58-40a4-8ef2-00072fb552b2
//
// A question edited since it was asked has no answer form (`answer`
// refuses it); the page shows its status and the move `adr show <file>`
// prints for it, the agent asking it again.
#[test]
fn page_shows_no_form_for_an_edited_question() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_minimal_terms(&dir, "widget");
    let record = write_record(&dir, "adrdom/decisions/example.md", TIER_Q1_RECORD);
    approve_record(&dir, "adrdom/decisions/example.md");
    assert!(adr(&dir, &["ask", record.to_str().unwrap()], &[], "").status.success());
    let q1_id = extract_ids(&std::fs::read_to_string(&record).unwrap())[0].clone();

    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let cookie = format!("adr_t={}", token);

    let form = format!("name=\"id\" value=\"{}\"", q1_id);
    let before = get(port, "/next", Some(&cookie));
    assert!(before.body.contains(&form), "body: {}", before.body);

    let text = std::fs::read_to_string(&record).unwrap();
    std::fs::write(&record, text.replacen("is undecided.", "is undecided, reworded.", 1)).unwrap();
    approve_record(&dir, "adrdom/decisions/example.md");

    let shown = get(port, "/next", Some(&cookie));
    assert!(!shown.body.contains(&form), "body: {}", shown.body);
    let lines = page_lines(&shown.body);
    assert!(lines.contains(&"[changed since asked] 1.1 What color is the widget?".to_string()), "{:#?}", lines);
    let file = cli_lines(&dir, &["show", "adrdom/decisions/example.md"]);
    let ask_move = file.iter().find(|l| l.starts_with("agent runs: adr ask ")).expect("the ask move");
    assert!(lines.contains(ask_move), "{:#?}", lines);

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
// assumed). The page's depends line for Q2, as `adr show <file>` prints
// it, still prints the heading words -- never the raw uuid the file
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
        next.body.contains("Stands on: 1.1 What color is the widget?"),
        "expected the page to turn the id back into the heading name: {}",
        next.body
    );
    assert!(!next.body.contains(&q1_id), "the page must never print the raw id: {}", next.body);

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
        "# Topic B\n\n### 1.1 Is B stand on A?\ndepends: {}\n\nProblem: p2 of the decision.\n\nStandard: s2.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
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

    let lines = page_lines(&resp.body);
    assert!(
        lines.contains(&"[changed] Topic B (adrdom/decisions/topic-b.md)".to_string()),
        "a hand-reverted depends line must still show as changed on the page: {:#?}",
        lines
    );
    assert!(
        lines.contains(&"[approved] Topic A (adrdom/decisions/topic-a.md)".to_string()),
        "A is genuinely approved and must show so: {:#?}",
        lines
    );

    let _ = child.kill();
    let _ = child.wait();
}

// adr: 7857d633-2055-47eb-857f-cb70da9fa7ba
//
// An act that wrote events and has findings on its input (an unban with one
// word banned and one not): the banner shows both, each as its own kind,
// and reads as no stop, since the act wrote.
#[test]
fn a_banner_with_events_and_input_findings_shows_both_and_is_no_stop() {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    assert!(adr(&dir, &["ban"], &[], "forbiddenword\n").status.success());

    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let cookie = format!("adr_t={}", token);
    let form = format!("lines=forbiddenword%0Anothingbanned&t={}", token);
    let resp = post(port, "/unban", Some(&cookie), &form);
    assert_eq!(resp.status, 302, "body: {}", resp.body);
    let followed = follow(port, &resp, &cookie);
    let _ = child.kill();
    let _ = child.wait();

    let banner_at = followed.body.find("<div class=\"banner").expect("a banner");
    let banner = &followed.body[banner_at..followed.body[banner_at..].find("</div>").map(|e| banner_at + e).unwrap()];
    assert!(banner.starts_with("<div class=\"banner\">"), "an act that wrote is no stop: {}", banner);
    let lines = page_lines(banner);
    assert!(lines.contains(&"unbanned: a word".to_string()), "{:?}", lines);
    assert!(lines.iter().any(|l| l.starts_with("unban: ")), "the input finding shows too: {:?}", lines);
}

// --- the restored pages: topic cards, one question at a time, decisions ---

const TWO_QUESTIONS: &str = r#"# Example

### 1.1 What is the unit of authority called?
depends: example

Problem: names leaked into decisions.

Standard: plain words.

- A) The authoritative sentence is a decision.
  Why: plainest word.
  Costs: two nouns.
- B) The whole unit is a block.
  Why: matches predecessors.
  Costs: names a region.

### 1.2 What is the second unit called?
depends: example, 1.1 What is the unit of authority called?=A

Problem: naming the second unit of a decision.

Standard: plain words.

- A) The second thing is a widget.
  Why: reasons.
  Costs: costs.
- B) The second thing is a gadget.
  Why: other reasons.
  Costs: other costs.
"#;

fn two_asked() -> std::path::PathBuf {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["decision", "example"]);
    let record = write_record(&dir, "adrdom/decisions/example.md", TWO_QUESTIONS);
    approve_record(&dir, "adrdom/decisions/example.md");
    let ask_out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(ask_out.status.success(), "ask failed: {}", stderr(&ask_out));
    dir
}

// adr: 7857d633-2055-47eb-857f-cb70da9fa7ba
//
// The overview is a card per topic file under the heading "Overview": the
// count of open questions, a link to the answer page, the file's status
// line, each asked question with its status line (a link where `answer`
// takes it, the move that says what it waits on where it does not), the
// terms card and the unban form.
#[test]
fn overview_is_one_card_per_topic_with_the_asked_questions() {
    let dir = two_asked();
    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let cookie = format!("adr_t={}", token);
    let resp = get(port, "/", Some(&cookie));
    let _ = child.kill();
    let _ = child.wait();
    assert_eq!(resp.status, 200, "body: {}", resp.body);

    let b = &resp.body;
    assert!(b.contains("<h1>Overview</h1>"), "{}", b);
    assert!(b.contains("2 open questions across 1 topic."), "{}", b);
    assert!(b.contains("<a class=\"primary\" href=\"/next\">Answer the next question</a>"), "{}", b);
    assert!(b.contains("<article class=\"topic\">") && b.contains("<h2>Example</h2>"), "{}", b);
    assert!(b.contains("<ol class=\"open-list\">"), "{}", b);
    // The first question is a link to its answer page; the second waits.
    assert!(b.contains("<a href=\"/next?show="), "{}", b);
    let lines = page_lines(b);
    assert!(
        lines.iter().any(|l| l.starts_with("not yet: \"1.2 What is the second unit called?\" is asked and waits for")),
        "{:#?}",
        lines
    );
    assert!(b.contains("<span class=\"label waiting\">waiting</span>"), "{}", b);
    // The terms card and the unban form.
    assert!(b.contains("<h2>Terms</h2>") && b.contains("Open the terms"), "{}", b);
    assert!(b.contains("<details class=\"unban\"><summary>Unban a phrase</summary>"), "{}", b);
    assert!(b.contains("action=\"/unban\""), "{}", b);
}

// adr: 7857d633-2055-47eb-857f-cb70da9fa7ba
//
// The answer page is one question at a time: its position among the open
// questions and their count, the options with Why and Costs, the terms its
// words pair beside it, what it stands on, the Answer, Refuse and Skip
// controls, and a link to each other open question. An answer moves on to
// the next question `answer` takes, with the act's event in the banner.
#[test]
fn next_is_one_question_with_the_count_and_the_others_linked() {
    let dir = two_asked();
    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let cookie = format!("adr_t={}", token);

    let resp = get(port, "/next", Some(&cookie));
    assert_eq!(resp.status, 200, "body: {}", resp.body);
    let b = &resp.body;
    assert!(b.contains("Question 1 of 2 open."), "{}", b);
    assert!(b.contains("<h1>") && b.contains("What is the unit of authority called?"), "{}", b);
    assert!(b.contains("<h2>2 open questions</h2>"), "the count of open questions: {}", b);
    assert!(b.contains("<span class=\"label waiting\">waiting</span>"), "the other question is listed: {}", b);
    assert!(b.contains("<dt>Why</dt><dd>plainest word.</dd>") && b.contains("<dt>Costs</dt><dd>two nouns.</dd>"), "{}", b);
    assert!(b.contains("<aside class=\"tq\">") && b.contains("<dt>decision</dt>"), "the terms aside: {}", b);
    assert!(b.contains("Stands on: example"), "{}", b);
    assert!(b.contains(">Refuse<") && b.contains("Skip this one") && b.contains("id=\"answer-button\""), "{}", b);
    // As on the old page the first option is picked, and the button names it.
    assert!(b.contains("id=\"opt-A\" value=\"A\" required checked"), "{}", b);
    assert!(b.contains("Answer <span id=\"answer-letter\">A</span>"), "{}", b);
    // The owner's move is the form, never the command text.
    assert!(!b.contains("class=\"move owner\""), "{}", b);

    // Answering moves on: the redirect carries the question to go after, and
    // the next page offers the question that was waiting, the event above.
    let id = extract_ids(&std::fs::read_to_string(dir.join("adrdom/decisions/example.md")).unwrap())[0].clone();
    let version = current_version(&dir, &id);
    let form = format!("id={}&v={}&answer=A&t={}", id, version, token);
    let answered = post(port, "/answer", Some(&cookie), &form);
    assert_eq!(answered.status, 302, "body: {}", answered.body);
    let location = header(&answered, "Location").unwrap().to_string();
    assert!(location.starts_with("/next?r=") && location.contains(&format!("after={}", id)), "{}", location);
    let next = follow(port, &answered, &cookie);
    let _ = child.kill();
    let _ = child.wait();
    assert!(next.body.contains("<div class=\"banner\">"), "{}", next.body);
    assert!(page_lines(&next.body).iter().any(|l| l.starts_with("answered: \"1.1 What is the unit of authority called?\" A")), "{}", next.body);
    assert!(next.body.contains("What is the second unit called?") && next.body.contains("Question 1 of 1 open."), "{}", next.body);
}

// adr: 7857d633-2055-47eb-857f-cb70da9fa7ba
//
// A stop on an answer shows the same question again, the findings that
// stopped it above it as a refused banner.
#[test]
fn a_stopped_answer_shows_the_same_question_with_its_findings() {
    let dir = two_asked();
    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let cookie = format!("adr_t={}", token);
    let id = extract_ids(&std::fs::read_to_string(dir.join("adrdom/decisions/example.md")).unwrap())[0].clone();
    let form = format!("id={}&v=deadbeef&answer=A&t={}", id, token);
    let stopped = post(port, "/answer", Some(&cookie), &form);
    let location = header(&stopped, "Location").unwrap().to_string();
    assert!(location.contains(&format!("show={}", id)), "{}", location);
    let page = follow(port, &stopped, &cookie);
    let _ = child.kill();
    let _ = child.wait();
    assert!(page.body.contains("<div class=\"banner refused\">"), "{}", page.body);
    assert!(page.body.contains("What is the unit of authority called?"), "{}", page.body);
}

// adr: 644b43ce-be10-4e9a-a7d5-71b27c51e365
//
// `/decisions` lists every current decision under its topic: the status
// line with the letter picked, the sentence, and how many tests tie to it.
#[test]
fn decisions_page_lists_each_decision_with_its_status_and_tests() {
    let dir = two_asked();
    let id = extract_ids(&std::fs::read_to_string(dir.join("adrdom/decisions/example.md")).unwrap())[0].clone();
    assert!(adr_answer(&dir, &id, "A").status.success());
    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let resp = get(port, "/decisions", Some(&format!("adr_t={}", token)));
    let _ = child.kill();
    let _ = child.wait();
    assert_eq!(resp.status, 200, "body: {}", resp.body);
    let b = &resp.body;
    assert!(b.contains("<h1>Current decisions</h1>") && b.contains("1 decision."), "{}", b);
    assert!(b.contains("<dl class=\"decisions\">"), "{}", b);
    assert!(page_lines(b).contains(&"[answered A] 1.1 What is the unit of authority called?".to_string()), "{}", b);
    assert!(b.contains("<span class=\"sentence\">The authoritative sentence is a decision.</span><span class=\"meta\">untested"), "{}", b);
}

// --- the review round: pick order, removed files, move ids, keys ---

const THREE_FLAT: &str = r#"# Example

### 1.1 What is the first unit called?
depends: example

Problem: names leaked into decisions.

Standard: plain words.

- A) The first thing is a decision.
  Why: plainest word.
  Costs: two nouns.
- B) The first thing is a block.
  Why: matches predecessors.
  Costs: names a region.

### 1.2 What is the second unit called?
depends: example

Problem: naming the second unit of a decision.

Standard: plain words.

- A) The second thing is a widget.
  Why: reasons.
  Costs: costs.
- B) The second thing is a gadget.
  Why: other reasons.
  Costs: other costs.

### 1.3 What is the third unit called?
depends: example

Problem: naming the third unit of a decision.

Standard: plain words.

- A) The third thing is a cog.
  Why: reasons.
  Costs: costs.
- B) The third thing is a gear.
  Why: other reasons.
  Costs: other costs.
"#;

fn three_asked() -> (std::path::PathBuf, Vec<String>) {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["decision", "example"]);
    let record = write_record(&dir, "adrdom/decisions/example.md", THREE_FLAT);
    approve_record(&dir, "adrdom/decisions/example.md");
    let ask_out = adr(&dir, &["ask", record.to_str().unwrap()], &[], "");
    assert!(ask_out.status.success(), "ask failed: {}", stderr(&ask_out));
    let ids = extract_ids(&std::fs::read_to_string(&record).unwrap());
    (dir, ids)
}

/// The heading of the question the answer page is on.
fn focus_heading(body: &str) -> String {
    let h1 = body.split("<h1>").nth(1).expect("an h1").split("</h1>").next().unwrap();
    unhtml(h1).trim().to_string()
}

// adr: 7857d633-2055-47eb-857f-cb70da9fa7ba
//
// Skip moves on in the order of the file and comes back to the one skipped
// last; an answer moves on from the place of the question answered, though
// that question is no longer open, so a question skipped earlier does not
// come first.
#[test]
fn next_moves_on_from_the_place_of_the_question_and_a_skipped_one_comes_last() {
    let (dir, ids) = three_asked();
    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let cookie = format!("adr_t={}", token);

    assert!(focus_heading(&get(port, "/next", Some(&cookie)).body).contains("1.1 "));
    // Skip walks the file order and wraps to the first last.
    let after_1 = get(port, &format!("/next?after={}", ids[0]), Some(&cookie));
    assert!(focus_heading(&after_1.body).contains("1.2 "), "{}", after_1.body);
    let after_2 = get(port, &format!("/next?after={}", ids[1]), Some(&cookie));
    assert!(focus_heading(&after_2.body).contains("1.3 "), "{}", after_2.body);
    let after_3 = get(port, &format!("/next?after={}", ids[2]), Some(&cookie));
    assert!(focus_heading(&after_3.body).contains("1.1 "), "{}", after_3.body);

    // 1.1 skipped, 1.2 answered: the page goes on to 1.3, not back to 1.1.
    let form = format!("id={}&v={}&answer=A&t={}", ids[1], current_version(&dir, &ids[1]), token);
    let answered = post(port, "/answer", Some(&cookie), &form);
    assert_eq!(answered.status, 302, "body: {}", answered.body);
    let next = follow(port, &answered, &cookie);
    assert!(focus_heading(&next.body).contains("1.3 "), "{}", next.body);

    // The last in the file answered: the page wraps to the first open one.
    let form = format!("id={}&v={}&answer=A&t={}", ids[2], current_version(&dir, &ids[2]), token);
    let answered = post(port, "/answer", Some(&cookie), &form);
    let next = follow(port, &answered, &cookie);
    let _ = child.kill();
    let _ = child.wait();
    assert!(focus_heading(&next.body).contains("1.1 "), "{}", next.body);
}

// adr: 644b43ce-be10-4e9a-a7d5-71b27c51e365
//
// `/decisions` lists the current decisions of a topic file removed from
// disk too, from its approved version, as `show` lists them.
#[test]
fn decisions_page_lists_the_decisions_of_a_removed_topic_file() {
    let dir = two_asked();
    let record = dir.join("adrdom/decisions/example.md");
    let id = extract_ids(&std::fs::read_to_string(&record).unwrap())[0].clone();
    assert!(adr_answer(&dir, &id, "A").status.success());
    assert!(run_git(&dir, &["add", "-A"]).status.success());
    let c = run_git(&dir, &["commit", "-q", "--no-verify", "-m", "chore: the example topic"]);
    assert!(c.status.success(), "{}", String::from_utf8_lossy(&c.stderr));
    std::fs::remove_file(&record).unwrap();

    let shown = stdout(&adr(&dir, &["show"], &[], ""));
    assert!(shown.contains("[removed]") && shown.contains("1.1 What is the unit of authority called?"), "{}", shown);

    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let resp = get(port, "/decisions", Some(&format!("adr_t={}", token)));
    let _ = child.kill();
    let _ = child.wait();
    let b = &resp.body;
    assert!(b.contains("1 decision."), "{}", b);
    assert!(page_lines(b).contains(&"[answered A] 1.1 What is the unit of authority called?".to_string()), "{}", b);
    assert!(b.contains("The authoritative sentence is a decision."), "{}", b);
}

fn answer_script(body: &str) -> &str {
    let from = body.find("<script>").expect("the answer page carries its script");
    let to = body[from..].find("</script>").expect("script end");
    &body[from..from + to]
}

// adr: 7857d633-2055-47eb-857f-cb70da9fa7ba
//
// The keys of the answer page, read off the script it emits: a letter and S
// work while a radio, a button or a link holds focus (only a field that takes
// text swallows keys); Enter submits from the page itself only, never from a
// button or a link (Refuse refuses, Skip skips, natively) and not from a
// radio, where the browser's own Enter submits the form once.
#[test]
fn answer_page_keys_leave_enter_to_buttons_links_and_radios() {
    let dir = two_asked();
    let mut child = spawn_owner(&dir, &["serve", "--port", "0"]);
    let (port, token) = read_open_line(&mut child);
    let resp = get(port, "/next", Some(&format!("adr_t={}", token)));
    let _ = child.kill();
    let _ = child.wait();
    let b = &resp.body;
    let js = answer_script(b);

    // The first option stays picked and the button names it.
    assert!(b.contains("id=\"opt-A\" value=\"A\" required checked"), "{}", b);
    assert!(b.contains("Answer <span id=\"answer-letter\">A</span>"), "{}", b);

    // Only a text field swallows keys: neither INPUT as a whole nor a radio.
    assert!(js.contains("tag === 'TEXTAREA'") && js.contains("type !== 'radio'"), "{}", js);
    assert!(!js.contains("if (tag === 'INPUT' || tag === 'TEXTAREA')"), "{}", js);
    // The letter and S branches sit apart from the focus rule on Enter, so
    // they run with focus on a radio, a button or a link.
    let enter = js.find("key === 'ENTER'").expect("an Enter branch");
    let letters = js.find("key >= 'A'").expect("a letter branch");
    let skip = js.find("key === 'S'").expect("an S branch");
    assert!(letters < enter && enter < skip, "{}", js);
    // Enter does nothing on a button, a link or a radio: the browser acts.
    let enter_branch = &js[enter..skip];
    for tag in ["'BUTTON'", "'A'", "'INPUT'"] {
        assert!(enter_branch.contains(tag), "{}: {}", tag, enter_branch);
    }
    assert!(enter_branch.contains("return;"), "{}", enter_branch);
    // requestSubmit appears once, in the Enter branch after that guard: a
    // radio never submits twice.
    assert_eq!(js.matches("requestSubmit").count(), 1, "{}", js);
    assert!(enter_branch.find("return;").unwrap() < enter_branch.find("requestSubmit").unwrap(), "{}", enter_branch);
}
