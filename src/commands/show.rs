//! `adr show`: per topic file, each derived tier and its questions,
//! plainly. No HTML, no snapshot — reads the file and ledger directly.
use crate::blame;
use crate::depends;
use crate::ledger::{self, Ledger};
use crate::msg::{self, Actor};
use crate::record::{self, RecordFile, Section};
use crate::repo;
use crate::state;
use crate::terms::{self, Term};
use crate::view::{self, QState};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

pub fn run(record_arg: Option<PathBuf>) -> i32 {
    let root = match repo::find_root() {
        Some(r) => r,
        None => {
            eprintln!("{}", msg::not_a_repo("show", Actor::Agent));
            return 1;
        }
    };
    let ledger_path = root.join(repo::LEDGER_PATH);
    let led = ledger::read(&ledger_path);
    let terms_list: Vec<Term> = state::load_terms(&root).map(|tf| tf.terms).unwrap_or_default();
    // A depends `Id` entry is printed as words -- a heading in this file, a
    // term name, or another topic file's decision heading text, so every
    // heading id in the repo is looked up once, here.
    let all_headings = view::all_heading_texts(&root);
    // Same pre-pass `approve`/`check` run before reading `rf.tiers`
    // (`depends::normalize_cross_file_deps`): a heading-shaped `depends:`
    // entry naming a current decision's heading in another topic file is
    // rewritten in memory to that decision's id and the tiers re-derived,
    // so an unapproved file with such an entry still prints its real
    // tiers instead of nothing (the file-wide derive error a plain,
    // unresolved `Text` heading dep would otherwise earn). The ambiguous
    // case (the same heading text naming a current decision in more than
    // one other file) becomes this file's `derive_error`
    // (`DeriveError::AmbiguousHeading`) instead of tiers: `print_file`
    // already prints nothing but the topic line when `rf.tiers` is `None`,
    // so `show` never guesses which decision an ambiguous entry means --
    // `check` is still where the ambiguity itself is reported.
    let phrase_lookup = terms::phrase_lookup(&terms_list);
    // Every topic file, loaded once: the index needs all of them regardless
    // of which one `record_arg` asks to print, so they're loaded here
    // rather than inside `cross_file_heading_index` (which used to load
    // them all itself, then have the loop below load whichever of them it
    // was about to print all over again).
    let all_records: Vec<RecordFile> = record::load_all(&root);
    let cross_file_headings = depends::cross_file_heading_index(&all_records, &led);

    // Every heading id, any state, and every blame file (per "Does show
    // print a blame file?", 7b05f8df A): built once here, off the same
    // full `all_records` the `record_arg` filter below is about to
    // consume, and kept for both the "from:" line under every decision and
    // the blame sections printed after every topic file's own.
    let any_headings_all = blame::any_headings_with_id(&all_records);
    let known_ids = blame::known_heading_ids(&any_headings_all);
    let blame_files = blame::load_all(&root);
    // A record-id lookup independent of `record_arg`'s filter (`records`
    // below may end up holding only the one file asked for): blame
    // rendering always resolves against every topic file, not only the one
    // being printed.
    let blame_records: Vec<RecordFile> = record::load_all(&root);

    let mut records: Vec<RecordFile> = match &record_arg {
        Some(p) => {
            let rel = state::rel_path(&root, p);
            match all_records.into_iter().find(|rf| rf.rel == rel) {
                Some(rf) => vec![rf],
                // Not one of the repo's topic files (an unusual path
                // outside adrdom/decisions/, say): loaded directly, same as
                // before this pass.
                None => record::load(p).into_iter().collect(),
            }
        }
        None => all_records,
    };

    for rf in &mut records {
        depends::normalize_cross_file_deps(rf, &terms_list, &phrase_lookup, &cross_file_headings, &led);
        print_file(
            rf,
            &led,
            &terms_list,
            &all_headings,
            &blame_files,
            &any_headings_all,
            &known_ids,
        );
    }

    for bf in &blame_files {
        print_blame_file(bf, &led, &blame_records);
    }
    0
}

/// `# <topic>` then each derived tier as `tier N` followed by its
/// questions. A file whose tier derivation failed prints only its `#
/// <topic>` line; `adr check`/`adr ask`/`adr approve` are where the
/// derivation error itself is reported.
#[allow(clippy::too_many_arguments)]
fn print_file(
    rf: &RecordFile,
    led: &Ledger,
    terms_list: &[Term],
    all_headings: &HashMap<String, String>,
    blame_files: &[blame::BlameFile],
    any_headings_all: &HashMap<String, Vec<(String, String)>>,
    known_ids: &HashSet<String>,
) {
    println!("# {}", rf.topic.as_deref().unwrap_or(""));

    let Some(tiers) = &rf.tiers else {
        return;
    };
    let max_tier = tiers.iter().copied().max().unwrap_or(0);
    for t in 1..=max_tier {
        let idxs: Vec<usize> = (0..rf.sections.len()).filter(|&i| tiers[i] == t).collect();
        if idxs.is_empty() {
            continue;
        }
        println!("tier {}", t);
        for &i in &idxs {
            print_question(
                rf,
                &rf.sections[i],
                led,
                terms_list,
                all_headings,
                blame_files,
                any_headings_all,
                known_ids,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn print_question(
    rf: &RecordFile,
    sec: &Section,
    led: &Ledger,
    terms_list: &[Term],
    all_headings: &HashMap<String, String>,
    blame_files: &[blame::BlameFile],
    any_headings_all: &HashMap<String, Vec<(String, String)>>,
    known_ids: &HashSet<String>,
) {
    let state = view::qstate(led, sec);
    // Per "Does an answer carry the version of the question the owner
    // read?": an asked (`Open`), unanswered question also prints the
    // version `answer` and the page's POST both check against -- the
    // owner's next move is `adr answer <id>@<version> <letter>`.
    if state == QState::Open {
        let version = record::version_of(&rf.lines, sec);
        println!(
            "  [{} @{}] {}",
            state_label(state, sec),
            version,
            sec.heading_text
        );
    } else {
        println!("  [{}] {}", state_label(state, sec), sec.heading_text);
    }
    if !sec.depends.is_empty() {
        let entries: Vec<String> = sec
            .depends
            .iter()
            .map(|d| view::format_dep(d, rf, terms_list, all_headings))
            .collect();
        println!("    stands on: {}", entries.join(", "));
    }
    if state == QState::Answered {
        if let Some(s) = sec.chosen_sentence() {
            println!("    {}", s);
        }
    }
    // Per "Does show print a blame file?", 7b05f8df A: "under each
    // decision ... a line `from: <origin path> (<n> rows)` for every blame
    // file with a row naming it." Only a section with an id can be named
    // by any row at all.
    if let Some(id) = &sec.id {
        for (origin, n) in blame::origins_for_decision(blame_files, any_headings_all, known_ids, id) {
            println!("    from: {} ({} {})", origin, n, if n == 1 { "row" } else { "rows" });
        }
    }
}

/// One section per blame file (per "Does show print a blame file?",
/// 7b05f8df A): the origin path, whether it is approved or still awaiting
/// approval, the row count per destination kind, every dropped row with
/// its range and reason, and every decision row with its range, the
/// heading text, and the question's state -- the same state words
/// `state_label` already gives a topic section.
fn print_blame_file(bf: &blame::BlameFile, led: &Ledger, records: &[RecordFile]) {
    let Some(origin) = &bf.origin else {
        // A shape-broken blame file: `check` is where that is refused;
        // `show` just skips it rather than guessing at a header it never
        // parsed.
        return;
    };
    println!("# blame: {}", bf.rel);
    let content = record::join_lines(&bf.lines);
    let status = if state::blame_approved(led, &bf.rel, &content) {
        "approved"
    } else {
        "awaiting approval"
    };
    println!("  origin: {} ({})", origin, status);

    let counts = blame::counts_by_kind(&bf.rows);
    if !counts.is_empty() {
        let line: Vec<String> = counts.iter().map(|(k, n)| format!("{} {}", k.label(), n)).collect();
        println!("  {}", line.join(", "));
    }

    let any_headings = blame::any_headings_with_id(records);
    let known_ids = blame::known_heading_ids(&any_headings);
    for row in &bf.rows {
        let range = if row.start == row.end {
            row.start.to_string()
        } else {
            format!("{}-{}", row.start, row.end)
        };
        if let Some(value) = blame::decision_target(&row.dest) {
            let (heading, qstate_word) = heading_and_state(value, records, led, &any_headings, &known_ids);
            println!("  {}\tdecision: {} [{}]", range, heading, qstate_word);
        } else if let Some(reason) = classify_dropped(&row.dest, records, led, &any_headings, &known_ids) {
            println!("  {}\tdropped: {}", range, reason);
        } else {
            // A term/prose row: print as written -- no id to turn back
            // into a name for either kind.
            println!("  {}\t{}", range, row.dest.trim());
        }
    }
}

/// A `decision:` or `dropped: copy of <heading>` value's heading text and
/// question-state word (per "Does show print a blame file?", 7b05f8df A --
/// and the owner's follow-up that a copy row's target earns the same state
/// tag a decision row's does, e.g. `[superseded]`, `[answered A]`), turning
/// an id back into words the way `view::format_dep` does for a depends
/// entry. `("unknown", "unknown")`-shaped when `value` resolves to no
/// section at all.
fn heading_and_state(
    value: &str,
    records: &[RecordFile],
    led: &Ledger,
    any_headings: &HashMap<String, Vec<(String, String)>>,
    known_ids: &HashSet<String>,
) -> (String, String) {
    match blame::resolve_heading_value(value, any_headings, known_ids) {
        Some(id) => {
            let heading = blame::heading_text_for_id(records, &id)
                .map(str::to_string)
                .unwrap_or_else(|| id.clone());
            let word = records
                .iter()
                .flat_map(|rf| rf.sections.iter())
                .find(|s| s.id.as_deref() == Some(id.as_str()))
                .map(|sec| state_label(view::qstate(led, sec), sec))
                .unwrap_or_else(|| "unknown".to_string());
            (heading, word)
        }
        None => (value.to_string(), "unknown".to_string()),
    }
}

/// The reason text `print_blame_file` prints after `dropped: ` for one
/// row: `dead`/`detail` as written; a copy-of-heading target rendered as
/// its heading text plus the same `[state]` tag a decision row gets; a
/// copy-of-path target rendered as the path, unchanged, with no tag (a
/// path names no question to have a state at all). `None` when `dest` is
/// not a `dropped:` row.
fn classify_dropped(
    dest: &str,
    records: &[RecordFile],
    led: &Ledger,
    any_headings: &HashMap<String, Vec<(String, String)>>,
    known_ids: &HashSet<String>,
) -> Option<String> {
    let t = dest.trim();
    if t == "dropped: dead" {
        return Some("dead".to_string());
    }
    if t == "dropped: detail" {
        return Some("detail".to_string());
    }
    let target = blame::copy_target(t)?;
    if record::is_uuid(target) || target.trim_end().ends_with('?') {
        let (heading, word) = heading_and_state(target, records, led, any_headings, known_ids);
        return Some(format!("copy of {} [{}]", heading, word));
    }
    Some(format!("copy of {}", blame::render_copy_target(target, records)))
}

/// The state word `show` prints inside `[...]`: `answered` also carries
/// the chosen letter (`answered A`), every other state is the bare word.
fn state_label(state: QState, sec: &Section) -> String {
    match state {
        QState::Skeleton => "skeleton".to_string(),
        QState::Unasked => "unasked".to_string(),
        QState::Open => "open".to_string(),
        QState::Answered => match sec.answer_letter() {
            Some(c) => format!("answered {}", c),
            None => "answered".to_string(),
        },
        QState::Refused => "refused".to_string(),
        QState::Superseded => "superseded".to_string(),
        QState::Withdrawn => "withdrawn".to_string(),
    }
}
