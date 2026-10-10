// "Does the tool colour what it prints to a terminal?" A and "What does the
// tool add to the shape of what it prints to a terminal?" A: the built
// binary run under a pseudo-terminal colours its lines and groups them with
// blank lines and headers; the same commands piped print plain, one line
// per item; NO_COLOR and TERM=dumb take the colour away and leave the shape.
mod common;

use common::*;
use std::path::Path;
use std::process::{Command, Stdio};

const REL: &str = "adrdom/decisions/example.md";

const ONE_QUESTION: &str = r#"# Example

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
"#;

fn script_present() -> bool {
    Command::new("script").arg("--version").stdout(Stdio::null()).stderr(Stdio::null()).status().is_ok_and(|s| s.success())
}

/// A repository with one question planned and not yet approved and a draft with a finding: `show` and
/// `check` print questions, findings and moves.
fn draft() -> std::path::PathBuf {
    let dir = init_repo();
    assert!(adr(&dir, &["install"], &[], "").status.success());
    approve_terms_with(&dir, &["decision", "example"]);
    write_record(&dir, REL, ONE_QUESTION);
    plan_record(&dir, REL);
    // A loaded word in a draft: a finding for check to print.
    write_record(
        &dir,
        "adrdom/decisions/other.md",
        "# Other\n\n### Does the example hold?\ndepends: example\n\nProblem: the example currently breaks.\n\nStandard: s.\n\n- A) Yes.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n",
    );
    dir
}

/// `adr <args>` under a pseudo-terminal, with `envs` set; the output with
/// the terminal's carriage returns dropped.
fn on_terminal(dir: &Path, args: &[&str], envs: &[(&str, &str)]) -> String {
    let mut cmd = scrubbed(Command::new("script"));
    let line = format!("{} {}", adr_bin().display(), args.join(" "));
    cmd.current_dir(dir)
        .args(["-qec", &line, "/dev/null"])
        .env_remove("CLAUDECODE")
        .env_remove("ADR_OWNER_BYPASS")
        .env_remove("NO_COLOR")
        .env("TERM", "xterm")
        .env("ADR_FAKE_MODEL", "1")
        .stdin(Stdio::null());
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let out = cmd.output().expect("script should run");
    String::from_utf8_lossy(&out.stdout).replace('\r', "")
}

fn piped(dir: &Path, args: &[&str]) -> String {
    stdout(&adr(dir, args, &[], ""))
}

#[test]
fn a_terminal_gets_colour_and_groups_and_a_pipe_gets_plain_lines() {
    // adr: b80e29dc-3f25-4aa8-8fc8-fa5ea57cd051
    // adr: f3c9e736-157c-4197-b49e-19e3674e4e0c
    if !script_present() {
        eprintln!("skipped: `script` is not installed");
        return;
    }
    let dir = draft();
    for args in [&["show"][..], &["show", REL][..], &["check"][..]] {
        let tty = on_terminal(&dir, args, &[]);
        assert!(tty.contains("\u{1b}["), "{:?} prints no colour on a terminal: {:?}", args, tty);
        assert!(tty.contains("\u{1b}[0m"), "{:?}: {:?}", args, tty);
        // `check` here prints one group (findings), headed and with nothing
        // before it to be separated from.
        let grouped = args[0] == "show";
        assert!(!grouped || tty.contains("\n\n"), "{:?} prints no blank line on a terminal: {:?}", args, tty);

        let plain = piped(&dir, args);
        assert!(!plain.contains('\u{1b}'), "{:?} piped carries an escape: {:?}", args, plain);
        assert!(!plain.contains("\n\n") && !plain.starts_with('\n'), "{:?} piped has a blank line: {:?}", args, plain);
        assert!(!plain.lines().any(|l| l == "Findings" || l == "Moves"), "{:?} piped has a header: {:?}", args, plain);

        // NO_COLOR: no escape, the shape stays.
        let no_colour = on_terminal(&dir, args, &[("NO_COLOR", "1")]);
        assert!(!no_colour.contains('\u{1b}'), "{:?} with NO_COLOR: {:?}", args, no_colour);
        assert!(!grouped || no_colour.contains("\n\n"), "{:?} with NO_COLOR: {:?}", args, no_colour);

        let dumb = on_terminal(&dir, args, &[("TERM", "dumb")]);
        assert!(!dumb.contains('\u{1b}'), "{:?} with TERM=dumb: {:?}", args, dumb);

        // Without the blank lines and headers the terminal's lines are the
        // piped lines.
        let stripped: Vec<&str> =
            no_colour.lines().filter(|l| !l.is_empty() && *l != "Findings" && *l != "Moves").collect();
        assert_eq!(stripped, plain.lines().collect::<Vec<_>>(), "{:?}", args);
    }
    let check = on_terminal(&dir, &["check"], &[("NO_COLOR", "1")]);
    assert_eq!(check.lines().next(), Some("Findings"), "{:?}", check);
    let show = on_terminal(&dir, &["show"], &[("NO_COLOR", "1")]);
    assert!(show.lines().any(|l| l == "Moves"), "{:?}", show);
    // A refusal on standard error is shaped like any findings group.
    let refusal = on_terminal(&dir, &["show", "adrdom/decisions/nosuch.md"], &[("NO_COLOR", "1")]);
    assert_eq!(refusal.lines().next(), Some("Findings"), "{:?}", refusal);
}
