// adr: 7857d633-2055-47eb-857f-cb70da9fa7ba
//
// "Does the ui show what the terminal prints?" A: the page renders the same
// statuses, findings, moves and events the commands print, from the one
// computation; the page shows nothing the terminal cannot. At each status
// of a fixture repository every line a page shows is a line `adr show`,
// `adr show --all` or `adr check` prints for the same records, and every
// line `adr show` prints is on the overview; after an act from the page,
// the lines it shows are the lines the command prints for the same act on
// the same records, one per ledger row it wrote.
mod common;

use common::*;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpStream;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::Duration;

const REL: &str = "adrdom/decisions/example.md";

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

struct Page {
    child: Child,
    port: u16,
    token: String,
}

impl Page {
    fn start(dir: &Path) -> Page {
        let mut child = Command::new(adr_bin())
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

    fn request(&self, head: &str, body: &str) -> (u16, String, String) {
        let mut stream = TcpStream::connect(("127.0.0.1", self.port)).unwrap();
        stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
        let req = format!(
            "{}\r\nHost: 127.0.0.1\r\nConnection: close\r\nCookie: adr_t={}\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\n\r\n{}",
            head,
            self.token,
            body.len(),
            body
        );
        stream.write_all(req.as_bytes()).unwrap();
        let mut raw = String::new();
        stream.read_to_string(&mut raw).unwrap();
        let (head, body) = raw.split_once("\r\n\r\n").unwrap_or((raw.as_str(), ""));
        let status = head.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
        let location = head
            .lines()
            .find_map(|l| l.strip_prefix("Location: "))
            .unwrap_or("")
            .trim()
            .to_string();
        (status, location, body.to_string())
    }

    /// The lines of the page at `path`.
    fn lines(&self, path: &str) -> Vec<String> {
        let (status, _, body) = self.request(&format!("GET {} HTTP/1.1", path), "");
        assert_eq!(status, 200, "{}", body);
        page_lines(&body)
    }

    /// Posts `form` (the token added) to `path` and returns the lines of the
    /// page the act leads to.
    fn act(&self, path: &str, form: &str) -> Vec<String> {
        let form = format!("{}&t={}", form, self.token);
        let (status, location, body) = self.request(&format!("POST {} HTTP/1.1", path), &form);
        assert_eq!(status, 302, "{}", body);
        self.lines(&location)
    }
}

impl Drop for Page {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The text of every line the page renders (one `<p class=...>` per line),
/// tags dropped, entities read back, without its indent.
fn page_lines(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = body;
    while let Some(i) = rest.find("<p class=\"") {
        let after = &rest[i..];
        let end = after.find("</p>").unwrap_or(after.len());
        let mut text = String::new();
        let mut in_tag = false;
        for c in after[..end].chars() {
            match c {
                '<' => in_tag = true,
                '>' => in_tag = false,
                c if !in_tag => text.push(c),
                _ => {}
            }
        }
        let text = text
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#39;", "'")
            .replace("&amp;", "&");
        out.push(text.trim_start().to_string());
        rest = &after[end..];
    }
    out
}

/// The lines `adr <args>` prints, each without its indent: standard output,
/// or standard error with `err`.
fn cli(dir: &Path, args: &[&str], owner: bool, err: bool) -> Vec<String> {
    let env: &[(&str, &str)] = if owner { &[("ADR_OWNER_BYPASS", "1")] } else { &[] };
    let out = adr(dir, args, env, "");
    let text = if err { stderr(&out) } else { stdout(&out) };
    text.lines().map(|l| l.trim_start().to_string()).filter(|l| !l.is_empty()).collect()
}

/// Every line a command that writes nothing prints for the records of `dir`:
/// `adr show`, `adr show --all` and `adr check`.
fn terminal(dir: &Path) -> Vec<String> {
    let mut lines = cli(dir, &["show"], false, false);
    lines.extend(cli(dir, &["show", "--all"], false, false));
    lines.extend(cli(dir, &["check"], false, false));
    lines
}

/// Every page shows only lines the terminal prints, and the overview shows
/// every line `adr show` prints.
fn assert_page_is_terminal(dir: &Path, page: &Page) {
    let terminal = terminal(dir);
    for path in ["/", "/next", "/terms"] {
        for l in page.lines(path) {
            assert!(terminal.contains(&l), "{} shows {:?}, which no command prints: {:#?}", path, l, terminal);
        }
    }
    let overview = page.lines("/");
    for l in cli(dir, &["show"], false, false) {
        assert!(overview.contains(&l), "/ lacks {:?}: {:#?}", l, overview);
    }
}

fn ledger_rows(dir: &Path) -> usize {
    std::fs::read_to_string(dir.join("adrdom/state/ledger.tsv")).unwrap_or_default().lines().count()
}

/// A repository with the terms approved and the two questions written and
/// planned, the plan awaiting the owner's approval.
fn draft() -> std::path::PathBuf {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["decision", "example"]);
    write_record(&dir, REL, TWO_QUESTIONS);
    plan_record(&dir, REL);
    dir
}

#[test]
fn every_page_shows_what_the_terminal_prints_at_each_status() {
    // adr: 7857d633-2055-47eb-857f-cb70da9fa7ba
    let dir = draft();
    let page = Page::start(&dir);

    // A plan awaiting approval: the owner's approve move, and the change.
    let show = cli(&dir, &["show"], false, false);
    assert!(show.contains(&format!("owner runs: adr approve {}", REL)), "{:?}", show);
    assert_page_is_terminal(&dir, &page);

    // Approved and asked: the owner's answer move, and what waits.
    let approve = adr(&dir, &["approve", REL], &[("ADR_OWNER_BYPASS", "1")], "");
    assert!(approve.status.success(), "{}", stderr(&approve));
    assert!(adr(&dir, &["ask", REL], &[], "").status.success());
    assert_page_is_terminal(&dir, &page);
    assert!(page.lines("/next").iter().any(|l| l.starts_with("not yet: ")), "{:#?}", page.lines("/next"));

    // Answered and untested: check's line with its move is on the page.
    let id = extract_ids(&std::fs::read_to_string(dir.join(REL)).unwrap())[0].clone();
    assert!(adr_answer(&dir, &id, "A").status.success());
    let check = cli(&dir, &["check"], false, false);
    assert!(check.iter().any(|l| l.starts_with("untested: ")), "{:?}", check);
    assert_page_is_terminal(&dir, &page);
    let overview = page.lines("/");
    for l in &check {
        assert!(overview.contains(l), "/ lacks check's {:?}", l);
    }

    // A finding: a loaded word in a new draft is check's note on the page.
    write_record(
        &dir,
        "adrdom/decisions/other.md",
        "# Other\n\n### Does the example hold?\ndepends: example\n\nProblem: the example currently breaks.\n\nStandard: s.\n\n- A) Yes.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n",
    );
    let check = cli(&dir, &["check"], false, false);
    assert!(check.iter().any(|l| l.contains("currently")), "{:?}", check);
    assert_page_is_terminal(&dir, &page);
}

#[test]
fn an_act_from_the_page_shows_the_lines_the_command_prints() {
    // adr: 7857d633-2055-47eb-857f-cb70da9fa7ba
    // Two repositories with the same records: one acts from the page, the
    // other from the terminal.
    let on_page = draft();
    let in_terminal = draft();
    let page = Page::start(&on_page);

    let before = ledger_rows(&on_page);
    let shown = page.act("/approve", &format!("file={}", REL));
    let printed = cli(&in_terminal, &["approve", REL], true, false);
    assert!(!printed.is_empty());
    assert_eq!(&shown[..printed.len()], &printed[..], "the page leads with the act's lines");
    assert_eq!(printed.len(), ledger_rows(&on_page) - before, "one line per ledger row written");

    assert!(adr(&on_page, &["ask", REL], &[], "").status.success());
    assert!(adr(&in_terminal, &["ask", REL], &[], "").status.success());
    let id_of = |dir: &Path| extract_ids(&std::fs::read_to_string(dir.join(REL)).unwrap())[0].clone();
    let (page_id, cli_id) = (id_of(&on_page), id_of(&in_terminal));

    let before = ledger_rows(&on_page);
    let shown = page.act("/answer", &format!("id={}&v={}&answer=A", page_id, current_version(&on_page, &page_id)));
    let printed = cli(&in_terminal, &["answer", &format!("{}@{}", cli_id, current_version(&in_terminal, &cli_id)), "A"], true, false);
    assert_eq!(printed.len(), 1, "{:?}", printed);
    assert_eq!(&shown[..1], &printed[..]);
    assert_eq!(ledger_rows(&on_page) - before, 1);

    // A stop: the same answer again, the same findings both ways.
    let shown = page.act("/answer", &format!("id={}&v={}&answer=B", page_id, current_version(&on_page, &page_id)));
    let stopped = cli(&in_terminal, &["answer", &format!("{}@{}", cli_id, current_version(&in_terminal, &cli_id)), "B"], true, true);
    assert!(!stopped.is_empty());
    let shown: Vec<String> = shown.into_iter().take(stopped.len()).map(|l| l.replace(&page_id, &cli_id)).collect();
    assert_eq!(shown, stopped);
}
