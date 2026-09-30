// adr: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
//
// "Does a refusal name the command that clears it?": src/msg.rs's own unit
// test (`every_refusal_names_its_next_move`) proves that every refusal or
// failure *built through `msg::`* carries a next-move clause, because
// `msg::`'s private `refuse()` is the one place that assembles the
// "<what>; <next>" text and every refusal/failure function in that module
// routes through it (a plain note, with no next move to name, does not).
// That leaves one gap a unit test inside msg.rs cannot see: a command that
// prints its own ad hoc refusal text instead of calling into `msg::` at
// all. This test closes that gap from the outside, by reading the source
// of every command and of src/actions.rs (the shared refusal-building code
// those commands call into) and checking that every `eprintln!` there
// either calls a `msg::` function directly, or passes through a single
// identifier already verified (see VERIFIED_PASSTHROUGHS below) to carry
// text `msg::` built.
//
// This is a text scan, not a borrow-checked data-flow analysis: it cannot
// prove a passthrough identifier's provenance by itself. What it does
// prove is that the set of "not obviously msg::-built" call sites is
// exactly the small, named set below -- so a new eprintln! that hardcodes
// its own message, anywhere under src/commands or in src/actions.rs, fails
// this test until it is either routed through msg:: or added here with a
// reviewed justification.

use std::path::{Path, PathBuf};

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap_or_else(|e| panic!("read_dir {}: {}", dir.display(), e)) {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
            out.push(path);
        }
    }
}

/// Every `eprintln!( ... )` call in `src`, as its full paren-balanced text
/// (so a call written across several lines comes back whole).
fn find_eprintln_calls(src: &str) -> Vec<String> {
    let mut calls = Vec::new();
    let mut from = 0usize;
    let bytes = src.as_bytes();
    while let Some(rel) = src[from..].find("eprintln!") {
        let start = from + rel;
        let Some(open_rel) = src[start..].find('(') else {
            break;
        };
        let open = start + open_rel;
        let mut depth = 1i32;
        let mut j = open + 1;
        while depth > 0 && j < bytes.len() {
            match bytes[j] {
                b'(' => depth += 1,
                b')' => depth -= 1,
                _ => {}
            }
            j += 1;
        }
        calls.push(src[start..j].to_string());
        from = j;
    }
    calls
}

/// Bare single-identifier passthroughs -- `eprintln!("...: {}", <ident>)`
/// with no `msg::` call in sight -- hand-verified at the time this test was
/// written to already carry `msg::`-built text:
///
/// - `e` (unban.rs, approve.rs): the `Err(String)` from
///   `actions::do_unban`/`do_approve`, and every `Err` src/actions.rs
///   returns is itself built from `msg::` calls (directly, or by joining
///   several `msg::`-built lines with "\n").
/// - `line` (answer.rs): one line of that same kind of `Err(String)`,
///   printed line by line.
/// - `m` (ask.rs): one of `shape_msgs`, each `format!("ask: {}", r.message)`
///   over a `Refusal` whose `message` field is built with `msg::`, and one
///   of the refusals ask gathers, each pushed as the result of a `msg::`
///   call.
/// - `h` (ban.rs): a bare `file:line` location string, printed as detail
///   *after* the actual refusal (`msg::already_in_use()`) that already
///   named ban's next move -- not itself a refusal.
const VERIFIED_PASSTHROUGHS: &[&str] = &["e", "line", "m", "h"];

#[test]
fn every_eprintln_in_commands_and_actions_is_msg_built_or_a_verified_passthrough() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    collect_rs_files(&root.join("src/commands"), &mut files);
    files.push(root.join("src/actions.rs"));
    assert!(files.len() > 5, "expected to find the command files: {:?}", files);

    let mut violations = Vec::new();
    for file in &files {
        let src = std::fs::read_to_string(file).unwrap();
        for call in find_eprintln_calls(&src) {
            if call.contains("msg::") {
                continue;
            }
            // No direct msg:: call: the only shape allowed here is a
            // single bare identifier as the sole interpolated argument.
            let inner = call
                .trim_start_matches("eprintln!")
                .trim_start()
                .trim_start_matches('(')
                .trim_end_matches(')')
                .trim();
            let arg = inner.rsplit(',').next().unwrap_or("").trim();
            let has_comma = inner.contains(',');
            let is_bare_ident = arg
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
                && !arg.is_empty();
            if has_comma && is_bare_ident && VERIFIED_PASSTHROUGHS.contains(&arg) {
                continue;
            }
            violations.push(format!("{}: {}", file.display(), call.replace('\n', " ")));
        }
    }

    assert!(
        violations.is_empty(),
        "found eprintln! calls that neither call msg:: nor pass through a \
         verified identifier from VERIFIED_PASSTHROUGHS -- either route the \
         message through msg:: (so it carries a next-move clause) or, if \
         it genuinely already does via an identifier, add that identifier \
         to VERIFIED_PASSTHROUGHS with a comment showing why:\n{}",
        violations.join("\n")
    );
}
