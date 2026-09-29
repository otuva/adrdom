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

/// `all` is the `--all` flag. With no topic file and no flag `show` prints
/// the short listing: each topic file's current decisions, heading and
/// answer, and its next moves. With a topic file, or with `--all`, it
/// prints the whole listing: every question in any state, the order they
/// are asked in, what each stands on and is stood on by, what replaced a
/// superseded one, the origins, and the blame files.
pub fn run(record_arg: Option<PathBuf>, all: bool) -> i32 {
    let whole = all || record_arg.is_some();
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
    let (all_records, unreadable_topic_files) =
        record::load_all_via(&root, &repo::Source::WorkingTree);
    let cross_file_headings = depends::cross_file_heading_index(&all_records, &led);

    // Every heading id, any state, and every blame file (per "Does show
    // print a blame file?", 7b05f8df A): built once here, off the same
    // full `all_records` the `record_arg` filter below is about to
    // consume, and kept for both the "from:" line under every decision and
    // the blame sections printed after every topic file's own.
    let (any_headings_all, known_ids) = blame::heading_ids(&all_records);
    let (blame_files, unreadable_blame_files) = blame::load_all_with_errors(&root);
    // A record-id lookup independent of `record_arg`'s filter (`records`
    // below may end up holding only the one file asked for): blame
    // rendering always resolves against every topic file, not only the one
    // being printed. Cloned rather than loaded a second time off disk.
    let blame_records: Vec<RecordFile> = all_records.clone();

    let mut records: Vec<RecordFile> = match &record_arg {
        Some(p) => {
            let rel = state::rel_path(&root, p);
            match all_records.into_iter().find(|rf| rf.rel == rel) {
                Some(rf) => vec![rf],
                // Not one of the repo's topic files (an unusual path
                // outside adrdom/decisions/, say): loaded directly, same as
                // before this pass.
                None => match record::load(p) {
                    Ok(rf) => vec![rf],
                    Err(e) => {
                        eprintln!(
                            "show: {}",
                            msg::read_failed(
                                &rel,
                                e,
                                msg::Next::agent("names a topic file that can be read as text")
                            )
                        );
                        return 1;
                    }
                },
            }
        }
        None => all_records,
    };
    // A topic file that cannot be read is named where it would have been
    // printed; with a file argument only that file is printed, and the one
    // asked for is refused above when it is the one that cannot be read.
    if record_arg.is_none() {
        for (f, e) in &unreadable_topic_files {
            println!("{}", msg::show_topic_unreadable(&state::rel_path(&root, f), e));
        }
    }

    let tests = crate::commands::check::test_index(&root, &repo::Source::WorkingTree);
    for rf in &mut records {
        // Read off the file as loaded, before the pass below rewrites its
        // depends entries in memory (see `view::snapshot`).
        let plan_ok = state::plan_approved(&led, rf, &rf.rel) && state::context_approved(&led, rf, &rf.rel);
        depends::normalize_cross_file_deps(rf, &terms_list, &phrase_lookup, &cross_file_headings, &led);
        if whole {
            print_file(
                rf,
                &blame_records,
                &led,
                &terms_list,
                &all_headings,
                &blame_files,
                &any_headings_all,
                &known_ids,
            );
        } else {
            print_file_short(rf, &led);
        }
        let plan_refusals: Vec<String> = crate::actions::record_refusals(
            &rf.rel,
            rf,
            &terms_list,
            &phrase_lookup,
            &cross_file_headings,
            &all_headings,
            &led,
        )
        .into_iter()
        .map(|r| r.message)
        .collect();
        let next = view::next_moves(rf, &led, plan_ok, &plan_refusals, &tests);
        if !next.is_empty() {
            println!("{}", msg::next_heading());
            for line in next {
                println!("{}", msg::next_line(&line));
            }
        }
    }

    // Reuses `any_headings_all`/`known_ids` (built off `all_records`
    // above, before the clone): `blame_records` is that same set of
    // sections and ids, so the index would only be recomputed to the same
    // content -- built once and passed to every file printed below,
    // rather than rebuilt per file.
    if !whole {
        println!("{}", msg::show_whole_hint());
        return 0;
    }
    for bf in &blame_files {
        print_blame_file(bf, &led, &blame_records, &any_headings_all, &known_ids);
    }
    for (rel, e) in &unreadable_blame_files {
        println!("{}", msg::show_blame_unreadable(rel, e));
    }
    0
}

/// The short listing of one topic file: its topic line, then each current
/// decision as its heading and the answer the owner picked, in file order.
/// A question that is not a current decision is left to the next moves
/// under the file, which name every one that still has a move.
fn print_file_short(rf: &RecordFile, led: &Ledger) {
    println!("{}", msg::show_topic(rf.topic.as_deref().unwrap_or(""), &rf.rel));
    for sec in &rf.sections {
        if view::qstate(led, sec) != QState::Answered {
            continue;
        }
        println!("{}", msg::show_decision(&sec.heading_text));
        if let Some(s) = sec.chosen_sentence() {
            println!("{}", msg::show_chosen(&s));
        }
    }
}

/// `# <topic>` then each derived tier as `tier N` followed by its
/// questions. A file whose tier derivation failed prints only its `#
/// <topic>` line; `adr check`/`adr ask`/`adr approve` are where the
/// derivation error itself is reported.
#[allow(clippy::too_many_arguments)]
fn print_file(
    rf: &RecordFile,
    all: &[RecordFile],
    led: &Ledger,
    terms_list: &[Term],
    all_headings: &HashMap<String, String>,
    blame_files: &[blame::BlameFile],
    any_headings_all: &HashMap<String, Vec<(String, String)>>,
    known_ids: &HashSet<String>,
) {
    println!("{}", msg::show_topic(rf.topic.as_deref().unwrap_or(""), &rf.rel));

    let Some(tiers) = &rf.tiers else {
        return;
    };
    let max_tier = tiers.iter().copied().max().unwrap_or(0);
    for t in 1..=max_tier {
        let idxs: Vec<usize> = (0..rf.sections.len()).filter(|&i| tiers[i] == t).collect();
        if idxs.is_empty() {
            continue;
        }
        println!("{}", msg::show_tier(t));
        for &i in &idxs {
            print_question(
                rf,
                &rf.sections[i],
                all,
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

/// The questions whose depends line names `sec`, in the order of the topic
/// files, each by its heading and, for a question of another file, the
/// path of that file. A refused, superseded or withdrawn question is left
/// out: its depends line is a record of what it stood on, nothing rests on
/// `sec` through it any more. An entry written as a heading text names
/// `sec` in its own file, and in another file only while `sec` is a
/// current decision (per "What does a depends line name?").
fn stood_on_by(rf: &RecordFile, sec: &Section, all: &[RecordFile], led: &Ledger) -> Vec<String> {
    let heading = sec.heading_text.trim();
    let current = sec.id.as_deref().is_some_and(|id| state::is_current(led, id));
    let mut out = Vec::new();
    for other in all {
        let same_file = other.rel == rf.rel;
        for dependent in &other.sections {
            if (same_file && dependent.heading_text.trim() == heading) || state::is_dead(led, dependent) {
                continue;
            }
            let names_it = dependent.depends.iter().any(|d| match d {
                record::Dep::Id { id, .. } => sec.id.as_deref() == Some(id.as_str()),
                record::Dep::Text { text, .. } => text.trim() == heading && (same_file || current),
            });
            if names_it {
                out.push(if same_file {
                    dependent.heading_text.clone()
                } else {
                    msg::show_question_of(&dependent.heading_text, &other.rel)
                });
            }
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn print_question(
    rf: &RecordFile,
    sec: &Section,
    all: &[RecordFile],
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
    // No version for a question edited since it was asked: `answer` would
    // refuse it, and "next" names the agent's move, to ask it again.
    let version = (state == QState::Open && view::asked_text_stands(led, rf, sec))
        .then(|| record::version_of(&rf.lines, sec));
    println!(
        "{}",
        msg::show_question(&state_label(state, sec), version.as_deref(), &sec.heading_text)
    );
    if !sec.depends.is_empty() {
        let entries: Vec<String> = sec
            .depends
            .iter()
            .map(|d| view::format_dep(d, rf, terms_list, all_headings))
            .collect();
        println!("{}", msg::show_stands_on(&entries));
    }
    // The decisions this question replaces, as words: an entry the tool
    // has turned into an identifier is turned back into the heading.
    if !sec.supersedes.is_empty() {
        let names: Vec<String> = sec
            .supersedes
            .iter()
            .map(|e| all_headings.get(e).cloned().unwrap_or_else(|| e.clone()))
            .collect();
        println!("{}", msg::show_supersedes(&names));
    }
    let dependents = stood_on_by(rf, sec, all, led);
    if !dependents.is_empty() {
        println!("{}", msg::show_stood_on_by(&dependents));
    }
    if state == QState::Answered {
        if let Some(s) = sec.chosen_sentence() {
            println!("{}", msg::show_chosen(&s));
        }
    }
    // A superseded question names what took its place: the ledger's
    // `superseded` row holds the id of the question that replaced it.
    if state == QState::Superseded {
        let replaced_by = sec
            .id
            .as_deref()
            .and_then(|id| led.latest_state(id))
            .map(|row| row.detail.trim())
            .filter(|by| !by.is_empty());
        if let Some(by) = replaced_by {
            let name = all_headings.get(by).map(String::as_str).unwrap_or(by);
            println!("{}", msg::show_replaced_by(name));
        }
    }
    // Per "Does show print a blame file?", 7b05f8df A: "under each
    // decision ... a line `from: <origin path> (<n> rows)` for every blame
    // file with a row naming it." Only a section with an id can be named
    // by any row at all.
    if let Some(id) = &sec.id {
        for (origin, n) in blame::origins_for_decision(blame_files, any_headings_all, known_ids, id) {
            println!("{}", msg::show_from(&origin, n));
        }
    }
}

/// One section per blame file (per "Does show print a blame file?",
/// 7b05f8df A): the origin path, whether it is approved or still awaiting
/// approval, the row count per destination kind, every dropped row with
/// its range and reason, and every decision row with its range, the
/// heading text, and the question's state -- the same state words
/// `state_label` already gives a topic section.
fn print_blame_file(
    bf: &blame::BlameFile,
    led: &Ledger,
    records: &[RecordFile],
    any_headings: &HashMap<String, Vec<(String, String)>>,
    known_ids: &HashSet<String>,
) {
    println!("{}", msg::show_blame(&bf.rel));
    // A shape-broken blame file is still printed (7b05f8df A: "each blame
    // file"): its shape errors first, then whatever parsed. `check` is
    // where the shape error is refused.
    for e in &bf.shape_errors {
        println!("{}", msg::show_blame_shape(e));
    }
    let Some(origin) = &bf.origin else {
        return;
    };
    let content = record::join_lines(&bf.lines);
    let approved = state::blame_approved(led, &bf.rel, &content);
    println!("{}", msg::show_blame_origin(origin, approved));

    let counts = blame::counts_by_kind(&bf.rows);
    if !counts.is_empty() {
        let line: Vec<String> = counts
            .iter()
            .map(|(k, n)| msg::show_blame_count(k.label(), *n))
            .collect();
        println!("{}", msg::show_blame_counts(&line));
    }

    for row in &bf.rows {
        let range = if row.start == row.end {
            row.start.to_string()
        } else {
            format!("{}-{}", row.start, row.end)
        };
        if let Some(value) = blame::decision_target(&row.dest) {
            let (heading, qstate_word) = heading_and_state(value, records, led, &any_headings, &known_ids);
            println!("{}", msg::show_blame_decision(&range, &heading, &qstate_word));
        } else if let Some(reason) = classify_dropped(&row.dest, records, led, &any_headings, &known_ids) {
            println!("{}", msg::show_blame_dropped(&range, &reason));
        } else {
            // A term/prose row: print as written -- no id to turn back
            // into a name for either kind.
            println!("{}", msg::show_blame_row(&range, row.dest.trim()));
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
    match blame::classify(t) {
        Some(blame::Kind::DroppedDead) => return Some("dead".to_string()),
        Some(blame::Kind::DroppedDetail) => return Some("detail".to_string()),
        _ => {}
    }
    let target = blame::copy_target(t)?;
    if record::is_uuid(target) || target.trim_end().ends_with('?') {
        let (heading, word) = heading_and_state(target, records, led, any_headings, known_ids);
        return Some(msg::show_blame_copy_of(&heading, Some(&word)));
    }
    Some(msg::show_blame_copy_of(&blame::render_copy_target(target, records), None))
}

/// `msg::show_state` for a section.
fn state_label(state: QState, sec: &Section) -> String {
    msg::show_state(state, sec.answer_letter())
}
