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

const PAGES: [&str; 4] = ["/", "/next", "/decisions", "/terms"];

/// What the page shows of a line the terminal prints, by kind: a status, a
/// finding or an agent's or a waiting move as text; the owner's move as the
/// form or link that makes it; a topic's heading line as the card's title.
#[derive(PartialEq)]
enum Kind {
    Owner,
    Heading,
    Text,
}

fn kind(l: &str) -> Kind {
    if is_owner_move(l) {
        Kind::Owner
    } else if l.starts_with("# ") {
        Kind::Heading
    } else {
        Kind::Text
    }
}

/// A status or a move as `show --all` prints it, not a line of a record.
fn is_status_or_move(l: &str) -> bool {
    let bare = l.strip_prefix("+ ").or_else(|| l.strip_prefix("- ")).unwrap_or(l);
    bare.starts_with('[') || l.starts_with("agent ") || l.starts_with("not yet:") || is_owner_move(l)
}

/// Every page shows only lines the terminal prints; the overview shows every
/// line `adr show` and `adr check` print (an owner's move as its form or
/// link); and every status and move of `adr show --all` is on a page,
/// whichever, the owner's moves as the form or link that makes them and
/// never as the command's text.
fn assert_page_is_terminal(dir: &Path, page: &Page) {
    let terminal = terminal(dir);
    let words: Vec<String> = terminal.iter().map(|l| page_words(l)).collect();
    let mut every_page: Vec<String> = Vec::new();
    let mut bodies = String::new();
    for path in PAGES {
        let (status, _, body) = page.request(&format!("GET {} HTTP/1.1", path), "");
        assert_eq!(status, 200, "{}", body);
        assert!(!body.contains("class=\"move owner\""), "{} shows an owner move as text", path);
        for l in page_lines(&body) {
            assert!(words.contains(&l), "{} shows {:?}, which no command prints: {:#?}", path, l, terminal);
            every_page.push(l);
        }
        bodies.push_str(&body);
    }
    let (_, _, overview_body) = page.request("GET / HTTP/1.1", "");
    let overview = page_lines(&overview_body);
    let mut owed = cli(dir, &["show"], false, false);
    owed.extend(cli(dir, &["check"], false, false));
    for l in owed {
        match kind(&l) {
            Kind::Owner => assert_owner_move_shown(&l, &overview_body),
            Kind::Heading => {}
            Kind::Text => assert!(overview.contains(&page_words(&l)), "/ lacks {:?}: {:#?}", l, overview),
        }
    }
    for l in cli(dir, &["show", "--all"], false, false).into_iter().filter(|l| is_status_or_move(l)) {
        match kind(&l) {
            Kind::Owner => assert_owner_move_shown(&l, &bodies),
            _ => assert!(every_page.contains(&page_words(&l)), "no page shows {:?}: {:#?}", l, every_page),
        }
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

    // A refused question: its status and the agent's move on it are on the
    // page.
    let ids = extract_ids(&std::fs::read_to_string(dir.join(REL)).unwrap());
    let refused = adr_answer(&dir, &ids[1], "refuse");
    assert!(refused.status.success(), "{}", stderr(&refused));
    let show = cli(&dir, &["show", "--all"], false, false);
    assert!(show.iter().any(|l| l.starts_with("[refused] ")), "{:?}", show);
    assert!(show.iter().any(|l| l.starts_with("agent changes the plan: ")), "{:?}", show);
    assert_page_is_terminal(&dir, &page);

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
