use crate::terms;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct OptionRec {
    pub letter: char,
    pub sentence: Option<String>,
    pub line: usize,
}

/// A `depends:` entry's answer-set suffix (`=A`, `=A|B`, `=*`): the letters
/// of the target question's options under which the dependent is asked
/// (per "Does a depends entry carry the answers it holds under?"). `Any`
/// is `=*`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AnswerSet {
    Any,
    Letters(Vec<char>),
}

/// One `depends:` line, classified only by whether it is a uuid or still
/// words (per "Does a reference in a topic file carry the identifier of
/// what it names?"): `Id` is a uuid already written to the file, resolved by
/// lookup (a heading id in this file, a term id, or a decision id) rather
/// than by shape, since `approve` is the only command that mints an id and
/// rewrites a `depends:` line to one. `Text` is anything still unresolved:
/// heading-shaped (ends in `?`, optionally carrying an answer set) or
/// term-shaped otherwise, exactly as `approve` classifies it when rewriting.
/// Term/decision/heading resolution for an `Id` or a term-shaped `Text`
/// happens in the commands (`approve`/`ask`/`check`), not here: record.rs has
/// no ledger or terms access; a heading-shaped reference (either form) is
/// resolved locally, in memory, against this file's own sections (used by
/// `derive_tiers`/`dependents_of` so `show` can print tiers even before a
/// plan is approved).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dep {
    Id { id: String, set: Option<AnswerSet> },
    Text { text: String, set: Option<AnswerSet> },
}

impl Dep {
    /// The answer-set suffix carried by this entry, if any, regardless of
    /// whether it is still words or already an id.
    pub fn set(&self) -> Option<&AnswerSet> {
        match self {
            Dep::Id { set, .. } => set.as_ref(),
            Dep::Text { set, .. } => set.as_ref(),
        }
    }
}

/// Why `derive_tiers` could not assign every section a tier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeriveError {
    /// A heading `depends:` entry (`entry`) named no heading in the file,
    /// found on the section whose heading text is `section`.
    Dangling { section: String, entry: String },
    /// The heading texts of a cycle, in traversal order (not closed: the
    /// first name is not repeated at the end).
    Cycle(Vec<String>),
    /// A section's own heading text, found as one of its own heading
    /// `depends:` entries.
    SelfDep(String),
    /// A heading-shaped `depends:` entry (`entry`, on the section whose
    /// heading text is `section`) naming no heading in this file, but a
    /// current decision's heading text in more than one *other* topic file
    /// (`files`, sorted, deduped): set by
    /// `depends::normalize_cross_file_deps` in place of calling
    /// `derive_tiers` at all, so this file's one `derive_error` slot always
    /// carries the real reason (ambiguous, never guessed at) instead of the
    /// `Dangling` `derive_tiers` alone would otherwise report for an entry
    /// it has no cross-file knowledge of.
    AmbiguousHeading {
        section: String,
        entry: String,
        files: Vec<String>,
    },
}

#[derive(Debug, Clone)]
pub struct Section {
    pub start: usize,
    pub end: usize,
    pub heading_text: String,
    pub id: Option<String>,
    /// `supersedes: ` ids, comma-separated in the source, in file order (4.14).
    pub supersedes: Vec<String>,
    /// `unrelated: ` entries, comma-separated in the source, in file order
    /// ("Does every neighbour get a line under its heading?").
    pub unrelated: Vec<String>,
    /// `depends: ` entries, in file order (8.3/8.4).
    pub depends: Vec<Dep>,
    pub options: Vec<OptionRec>,
    pub answer_line: Option<usize>,
    pub answer_value: Option<String>,
    /// Errors from the parser: duplicate id line, an option
    /// with no decision sentence, and an answer naming a missing option.
    /// The option-count rules ("at most three" / "at least two") live in
    /// `shape_errors` instead, since `ask` and `approve` must refuse on
    /// them before ever touching the file, same as every other shape rule.
    pub parse_errors: Vec<String>,
    /// Errors from the topic-file-shape rules (4.9/4.7/3.4/8.3): stray
    /// lines in a body, a non-skeleton question missing its
    /// Problem/Standard lines, an option count outside two-to-three, an
    /// option missing its Why/Costs line, and a malformed `depends:` line.
    /// `ask` and `approve` both refuse on this slice (plus the file-level
    /// `RecordFile::shape_errors`) before doing anything else; `check`'s
    /// `parse` check reads this and `parse_errors` together.
    pub shape_errors: Vec<String>,
    /// True when the heading has no non-blank line before the next
    /// heading other than `depends:` lines: no body, never a failure --
    /// `approve` still gives a skeleton heading an id.
    pub is_skeleton: bool,
    /// Line index of this section's `terms: ` line, if any.
    pub terms_line: Option<usize>,
}

impl Section {
    pub fn answer_letter(&self) -> Option<char> {
        let v = self.answer_value.as_ref()?.trim();
        if v.chars().count() == 1 {
            let c = v.chars().next().unwrap();
            if self.options.iter().any(|o| o.letter == c) {
                return Some(c);
            }
        }
        None
    }

    /// True when `**Answer:** refused` (per "Does the owner answer a
    /// question with a refusal?"): answered for hashing and
    /// editing purposes (the text is frozen), unanswered for the tier rule
    /// and for `done`.
    pub fn is_refused(&self) -> bool {
        self.answer_value
            .as_deref()
            .map(|v| v.trim() == "refused")
            .unwrap_or(false)
    }

    pub fn chosen_sentence(&self) -> Option<&str> {
        let c = self.answer_letter()?;
        self.options
            .iter()
            .find(|o| o.letter == c)
            .and_then(|o| o.sentence.as_deref())
    }
}

#[derive(Clone)]
pub struct RecordFile {
    pub path: PathBuf,
    /// The path as printed: relative to the repo root with forward slashes
    /// when loaded through `load_all`/`load_all_via`, otherwise the path as
    /// given (`load`).
    pub rel: String,
    pub lines: Vec<String>,
    pub sections: Vec<Section>,
    /// The h1 topic text, or `None` if the file has no h1 (a shape error).
    pub topic: Option<String>,
    /// [start, end) line range of the context prose: everything between
    /// the h1 and the first `### ` question heading (or end of file, if no
    /// h1/questions).
    pub context_range: (usize, usize),
    /// File-level shape errors: missing/duplicate h1, text
    /// before h1, a `## ` tier heading — tiers are derived now, never
    /// written by hand.
    pub shape_errors: Vec<String>,
    /// Each section's 1-based tier (index-aligned with `sections`), filled
    /// when `derive_tiers` succeeds; `None` when `derive_error` is set
    /// instead. Mutually exclusive with `derive_error`.
    pub tiers: Option<Vec<usize>>,
    /// Why tier derivation failed, when it did.
    pub derive_error: Option<DeriveError>,
}

impl RecordFile {
    pub fn tier_of(&self, idx: usize) -> Option<usize> {
        self.tiers.as_ref().and_then(|t| t.get(idx).copied())
    }

    /// The section index a heading-shaped `depends:` entry names in this
    /// file, if any: an `Id` entry is matched by the target section's id
    /// (the form every heading dep takes once the plan is approved); a
    /// `Text` entry is matched only when heading-shaped (ends in `?`)
    /// against the target's heading text (the pre-approval form, so `show`
    /// can still derive tiers before `approve` has minted anything). `None`
    /// for a term/decision reference, or a `Text` entry naming no heading in
    /// this file (a dangling reference, reported elsewhere).
    pub fn dep_target(&self, d: &Dep) -> Option<usize> {
        match d {
            Dep::Id { id, .. } => self.sections.iter().position(|s| s.id.as_deref() == Some(id.as_str())),
            Dep::Text { text, .. } => {
                let t = text.trim();
                if !t.ends_with('?') {
                    return None;
                }
                self.sections.iter().position(|s| s.heading_text.trim() == t)
            }
        }
    }

    /// Sections whose heading `depends:` entry names section `idx`'s
    /// heading (by id once approved, by heading text before), paired with
    /// the set they declare on that entry, if any (per "What does a
    /// depends line name?", updated by "Does a reference in a topic file
    /// carry the identifier of what it names?": dependents_of keys on
    /// heading ids once they exist).
    pub fn dependents_of(&self, idx: usize) -> Vec<(usize, Option<&AnswerSet>)> {
        let mut out = Vec::new();
        if self.sections.get(idx).is_none() {
            return out;
        }
        for (i, sec) in self.sections.iter().enumerate() {
            for d in &sec.depends {
                if self.dep_target(d) == Some(idx) {
                    out.push((i, d.set()));
                }
            }
        }
        out
    }
}

pub fn normalize(content: &str) -> String {
    content.replace("\r\n", "\n")
}

pub fn split_lines(content: &str) -> Vec<String> {
    normalize(content)
        .split('\n')
        .map(|s| s.to_string())
        .collect()
}

pub fn join_lines(lines: &[String]) -> String {
    lines.join("\n")
}

pub fn load(path: &Path) -> std::io::Result<RecordFile> {
    let content = fs::read_to_string(path)?;
    let lines = split_lines(&content);
    let rel = path.display().to_string();
    Ok(build(path.to_path_buf(), rel, lines))
}

/// Every topic file under `root` (`repo::list_decision_files`), loaded off
/// the filesystem by `load_all_via` building the `RecordFile` directly; a
/// file that fails to load is left out silently, matching every caller of
/// this before it except `check` (`load_all_via`, which needs to name each
/// one that failed and why).
pub fn load_all(root: &Path) -> Vec<RecordFile> {
    load_all_via(root, &crate::repo::Source::WorkingTree).0
}

/// Like `load_all`, but through `source` (so `check` at a staged or
/// working-tree commit reads exactly what it is checking, not always the
/// filesystem -- `repo::list_decision_files_via`/`Source::read`), and
/// keeping every file that failed to read instead of dropping it, each
/// paired with why.
pub fn load_all_via(root: &Path, source: &crate::repo::Source) -> (Vec<RecordFile>, Vec<(PathBuf, String)>) {
    let mut records = Vec::new();
    let mut errors = Vec::new();
    for f in crate::repo::list_decision_files_via(root, source) {
        let rel = crate::state::rel_path(root, &f);
        match source.read(root, &rel) {
            Some(content) => records.push(build(f, rel, split_lines(&content))),
            None => errors.push((f, "it holds no text the tool can read".to_string())),
        }
    }
    (records, errors)
}

/// Rebuilds a `RecordFile` from an in-memory line buffer (re-parsing
/// sections, shape, and tiers): used by `approve` to reparse after minting
/// heading ids, so the depends-rewrite pass sees each target's freshly
/// minted id.
pub(crate) fn build(path: PathBuf, rel: String, lines: Vec<String>) -> RecordFile {
    let sections = parse_sections(&lines);
    let (topic, context_range, shape_errors) = analyze_shape(&lines, &sections);
    let mut rf = RecordFile {
        path,
        rel,
        lines,
        sections,
        topic,
        context_range,
        shape_errors,
        tiers: None,
        derive_error: None,
    };
    match derive_tiers(&rf) {
        Ok(t) => rf.tiers = Some(t),
        Err(e) => rf.derive_error = Some(e),
    }
    rf
}

pub fn save(rf: &RecordFile) -> std::io::Result<()> {
    fs::write(&rf.path, join_lines(&rf.lines))
}

fn is_question_heading(line: &str) -> bool {
    line.starts_with("### ") && line.trim_end().ends_with('?')
}

/// The indices of the `### ` lines that do not end in `?`: a heading is a
/// question (per "What does a topic file hold?"), so such a line is
/// refused rather than read as prose.
pub fn non_question_heading_lines(lines: &[String]) -> Vec<usize> {
    lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.starts_with("### ") && !is_question_heading(l))
        .map(|(i, _)| i)
        .collect()
}

fn is_h1(line: &str) -> bool {
    line.starts_with("# ")
}

/// A `## ` line: no longer a valid tier heading (tiers are
/// derived from `depends:` lines), but still recognized so it can be
/// reported as a shape error instead of silently becoming stray text or
/// (worse) being mistaken for something else.
fn is_h2(line: &str) -> bool {
    line.starts_with("## ")
}

pub(crate) fn parse_option_line(line: &str) -> Option<(char, String)> {
    if !line.starts_with("- ") {
        return None;
    }
    let after_dash = &line[2..];
    let mut chars = after_dash.chars();
    let letter = chars.next()?;
    if !letter.is_ascii_uppercase() {
        return None;
    }
    let rest1 = chars.as_str();
    let rest = rest1.strip_prefix(") ")?;
    Some((letter, rest.to_string()))
}

fn extract_sentence(rest: &str) -> Option<String> {
    let chars: Vec<char> = rest.chars().collect();
    for (idx, c) in chars.iter().enumerate() {
        if *c == '.' || *c == '!' || *c == '?' {
            let next = chars.get(idx + 1);
            if next.is_none() || *next.unwrap() == ' ' {
                let sentence: String = chars[..=idx].iter().collect();
                return Some(sentence);
            }
        }
    }
    None
}

/// Which of the two option-detail lines a body line is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Detail {
    Why,
    Costs,
}

/// Recognizes a `  Why: ` or `  Costs: ` line and returns which kind it is
/// plus the text after the prefix. The one place both `scan_body` (which
/// counts these under each option) and `option_details` (which extracts
/// their text) recognize the prefix, so the two stay in sync.
pub(crate) fn detail_line(line: &str) -> Option<(Detail, &str)> {
    if let Some(v) = line.strip_prefix("  Why: ") {
        return Some((Detail::Why, v));
    }
    if let Some(v) = line.strip_prefix("  Costs: ") {
        return Some((Detail::Costs, v));
    }
    None
}

/// `"key: value"`, trimmed: `None` when `line` doesn't start with `key`
/// followed by a colon and a space. `terms::parse` keeps its own version of
/// this (`terms::kv`) rather than sharing it: a real `adrdom/terms.md`
/// field left empty is written with no space after the colon at all (a
/// bare `aliases:`), which this stricter form would treat as not naming
/// the field.
fn kv(line: &str, key: &str) -> Option<String> {
    let prefix = format!("{}: ", key);
    line.strip_prefix(&prefix).map(|v| v.trim().to_string())
}

fn answer_value(line: &str) -> Option<String> {
    line.strip_prefix("**Answer:** ")
        .map(|v| v.trim().to_string())
}

/// True when `s` (already trimmed) is a uuid: 8-4-4-4-12 hex, dashed
/// (upper or lower case -- the same shape `check`'s test-tie scan and the
/// page's uuid-stripping pass also match).
pub fn is_uuid(s: &str) -> bool {
    let bytes: Vec<char> = s.chars().collect();
    if bytes.len() != 36 {
        return false;
    }
    const DASH_POS: [usize; 4] = [8, 13, 18, 23];
    for (i, c) in bytes.iter().enumerate() {
        if DASH_POS.contains(&i) {
            if *c != '-' {
                return false;
            }
        } else if !c.is_ascii_hexdigit() {
            return false;
        }
    }
    true
}

/// `=A`, `=A|B`, `=*` after the `=`. `None` on anything else (bad set
/// syntax): an empty string, a part longer than one character, a
/// lowercase letter, or an empty part from `||` or a trailing `|`.
fn parse_answer_set(s: &str) -> Option<AnswerSet> {
    if s == "*" {
        return Some(AnswerSet::Any);
    }
    if s.is_empty() {
        return None;
    }
    let mut letters = Vec::new();
    for part in s.split('|') {
        let mut chars = part.chars();
        let c = chars.next()?;
        if chars.next().is_some() || !c.is_ascii_uppercase() {
            return None;
        }
        letters.push(c);
    }
    Some(AnswerSet::Letters(letters))
}

/// Splits a `depends:` line's whole value into its entries (per "Does a
/// reference line hold its whole list on one line?"): entries
/// are comma separated, an answer set stays on its own entry (`|` inside a
/// set, never `,`). Before approve, an entry may itself be heading text that
/// holds a comma: fragments are rejoined, longest run first, against
/// `headings` (every heading text in the file, gathered once up front by
/// the caller so a later heading can be named by an earlier one and vice
/// versa) -- `parse_depends_entry` on the rejoined text tells whether it is
/// heading-shaped (ends in `?`, once any `=set` tail is set aside) so this
/// never needs its own copy of that rule. A fragment run that matches no
/// heading falls back to one entry per raw fragment, same as a plain
/// comma-separated list with no embedded commas.
fn parse_depends_line(value: &str, headings: &[String]) -> Vec<Result<Dep, String>> {
    let pieces: Vec<String> = value.split(',').map(|s| s.trim().to_string()).collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < pieces.len() {
        let mut chosen = i + 1;
        for j in (i + 2..=pieces.len()).rev() {
            let joined = pieces[i..j].join(", ");
            if let Ok(Dep::Text { text, .. }) = parse_depends_entry(&joined) {
                if headings.iter().any(|h| h.trim() == text.trim()) {
                    chosen = j;
                    break;
                }
            }
        }
        let joined = pieces[i..chosen].join(", ");
        out.push(parse_depends_entry(&joined));
        i = chosen;
    }
    out
}

/// Classifies one `depends:` entry (the trimmed text after `depends: `) into
/// `Id` (a uuid, with or without a trailing `=set`) or `Text` (words, with
/// or without a trailing `=set` on a heading-shaped base). `Err` names why
/// the entry is malformed, without the "depends: " prefix or line number
/// (the caller adds those).
fn parse_depends_entry(raw: &str) -> Result<Dep, String> {
    if raw.is_empty() {
        return Err("empty entry".to_string());
    }
    if let Some(eq_idx) = raw.rfind('=') {
        let base = raw[..eq_idx].trim_end();
        if is_uuid(base) || base.ends_with('?') {
            let set_str = &raw[eq_idx + 1..];
            return match parse_answer_set(set_str) {
                Some(set) => Ok(if is_uuid(base) {
                    Dep::Id {
                        id: base.to_string(),
                        set: Some(set),
                    }
                } else {
                    Dep::Text {
                        text: base.to_string(),
                        set: Some(set),
                    }
                }),
                None => Err("bad set syntax".to_string()),
            };
        }
    }
    if is_uuid(raw) {
        return Ok(Dep::Id {
            id: raw.to_string(),
            set: None,
        });
    }
    Ok(Dep::Text {
        text: raw.to_string(),
        set: None,
    })
}

/// Whether two successfully-parsed deps name the same target (ignoring any
/// answer set): used to catch the same entry repeated within one `depends:`
/// line, not a repeated `depends:` line (a second one is its own error).
fn dep_identity_eq(a: &Dep, b: &Dep) -> bool {
    match (a, b) {
        (Dep::Id { id: x, .. }, Dep::Id { id: y, .. }) => x == y,
        (Dep::Text { text: x, .. }, Dep::Text { text: y, .. }) => x == y,
        _ => false,
    }
}

pub fn parse_sections(lines: &[String]) -> Vec<Section> {
    // Every heading's text, gathered once before any body is scanned: a
    // `depends:` line's comma-rejoin (see `parse_depends_line`) may need to
    // match a heading that comes later in the file, or the section's own.
    let headings: Vec<String> = lines
        .iter()
        .filter(|l| is_question_heading(l))
        .map(|l| l["### ".len()..].trim().to_string())
        .collect();

    let mut sections = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = &lines[i];
        if is_question_heading(line) {
            let start = i;
            let end = find_end(lines, i + 1);
            sections.push(parse_question(lines, start, end, &headings));
            i = end;
        } else {
            i += 1;
        }
    }
    sections
}

fn find_end(lines: &[String], from: usize) -> usize {
    for (j, line) in lines.iter().enumerate().skip(from) {
        if is_question_heading(line) || is_h2(line) {
            return j;
        }
    }
    lines.len()
}

/// Everything a heading's body scan produces: the pre-existing bits
/// (id/supersedes/options/answer) plus the shape-rule bits (terms line,
/// depends entries, stray lines, Problem/Standard counts, and each
/// option's Why/Costs completeness).
struct BodyScan {
    id: Option<String>,
    id_count: usize,
    supersedes: Vec<String>,
    unrelated: Vec<String>,
    depends: Vec<Dep>,
    depends_errors: Vec<String>,
    /// "supersedes: second supersedes line, line N" / "terms: second terms
    /// line, line N": a82db5b7 holds every reference list on one line, so
    /// a second line of either is refused, the same as a second depends
    /// line, rather than silently ignored.
    reference_line_errors: Vec<String>,
    /// "terms: second terms line, line N": the tool writes that line.
    terms_errors: Vec<String>,
    terms_line: Option<usize>,
    options: Vec<OptionRec>,
    answer_line: Option<usize>,
    answer_value: Option<String>,
    option_errors: Vec<String>,
    stray_errors: Vec<String>,
    problem_count: usize,
    standard_count: usize,
    /// "option A missing Why" / "option B has two Costs lines", one entry
    /// per option that lacks exactly one non-empty Why/Costs line under it.
    option_shape_errors: Vec<String>,
    /// "second answer line, line N" for each `**Answer:**` line past the
    /// first one in the section: `answer_line`/`answer_val` always keep the
    /// first occurrence, so a later one can never overwrite them and hide
    /// itself from the "answer line comes last" check.
    answer_errors: Vec<String>,
}

fn scan_body(lines: &[String], start: usize, end: usize, headings: &[String]) -> BodyScan {
    let mut id = None;
    let mut id_count = 0;
    let mut supersedes: Vec<String> = Vec::new();
    let mut depends: Vec<Dep> = Vec::new();
    let mut depends_errors: Vec<String> = Vec::new();
    let mut depends_line_seen = false;
    let mut reference_line_errors: Vec<String> = Vec::new();
    let mut terms_errors: Vec<String> = Vec::new();
    let mut supersedes_line_seen = false;
    let mut unrelated: Vec<String> = Vec::new();
    let mut unrelated_line_seen = false;
    let mut terms_line = None;
    let mut options: Vec<OptionRec> = Vec::new();
    let mut answer_line = None;
    let mut answer_val = None;
    let mut option_errors = Vec::new();
    let mut stray_errors = Vec::new();
    let mut answer_errors = Vec::new();
    let mut problem_count = 0;
    let mut standard_count = 0;
    // Parallel to `options`: how many non-empty `  Why: `/`  Costs: ` lines
    // have been seen directly under each option so far.
    let mut why_counts: Vec<usize> = Vec::new();
    let mut costs_counts: Vec<usize> = Vec::new();
    // True while the previous non-blank line was an option or one of its
    // Why/Costs lines; a Why/Costs anywhere else is stray.
    let mut under_option = false;

    for (j, line) in lines.iter().enumerate().take(end).skip(start + 1) {
        if line.trim().is_empty() {
            continue;
        }
        // A `### ` line without `?` is refused once, by its own line
        // (`non_question_heading_lines`); it is not also a stray line.
        if line.starts_with("### ") {
            continue;
        }
        // A `depends:` with nothing after it names nothing: it counts as a
        // depends line with no entries, so the section is refused as having
        // no depends line (`record_refusals`) and nothing else about it.
        let empty_depends = is_bare_depends_line(line);
        if let Some(v) = kv(line, "depends").or_else(|| empty_depends.then(String::new)) {
            if v.is_empty() {
                depends_line_seen = true;
                continue;
            }
            if depends_line_seen {
                depends_errors.push(format!("depends: second depends line, line {}", j + 1));
                continue;
            }
            depends_line_seen = true;
            for res in parse_depends_line(&v, headings) {
                match res {
                    Ok(d) => {
                        if depends.iter().any(|existing| dep_identity_eq(existing, &d)) {
                            depends_errors.push(format!("depends: duplicate entry, line {}", j + 1));
                        } else {
                            depends.push(d);
                        }
                    }
                    Err(why) => {
                        depends_errors.push(format!("depends: {}, line {}", why, j + 1));
                    }
                }
            }
            continue;
        }
        if let Some(v) = kv(line, "id") {
            if id_count == 0 {
                id = Some(v);
            }
            id_count += 1;
            continue;
        }
        if let Some(v) = kv(line, "supersedes") {
            if supersedes_line_seen {
                reference_line_errors
                    .push(format!("supersedes: second supersedes line, line {}", j + 1));
                continue;
            }
            supersedes_line_seen = true;
            supersedes = v
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            continue;
        }
        if let Some(v) = kv(line, "unrelated") {
            if unrelated_line_seen {
                reference_line_errors
                    .push(format!("unrelated: second unrelated line, line {}", j + 1));
                continue;
            }
            unrelated_line_seen = true;
            unrelated = v
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            continue;
        }
        if kv(line, "terms").is_some() {
            if terms_line.is_some() {
                terms_errors.push(format!("terms: second terms line, line {}", j + 1));
                continue;
            }
            terms_line = Some(j);
            continue;
        }
        if kv(line, "Problem").is_some() {
            problem_count += 1;
            continue;
        }
        if kv(line, "Standard").is_some() {
            standard_count += 1;
            continue;
        }
        if let Some((letter, rest)) = parse_option_line(line) {
            let sentence = extract_sentence(&rest);
            if sentence.is_none() {
                option_errors.push(format!("option {} has no decision sentence", letter));
            } else {
                // 178a232e A's "each one sentence": a decision text holding
                // a second sentence is refused too -- `extract_sentence`
                // above only ever looks at the first one, so a count has to
                // be taken separately to catch the rest of the text.
                let count = terms::sentence_count(rest.trim_end());
                if count > 1 {
                    option_errors.push(format!(
                        "option {} decision text has {} sentences, more than one",
                        letter, count
                    ));
                }
            }
            options.push(OptionRec {
                letter,
                sentence,
                line: j,
            });
            why_counts.push(0);
            costs_counts.push(0);
            under_option = true;
            continue;
        }
        if let Some((kind, v)) = detail_line(line) {
            if under_option {
                if !v.trim().is_empty() {
                    let counts = match kind {
                        Detail::Why => &mut why_counts,
                        Detail::Costs => &mut costs_counts,
                    };
                    if let Some(c) = counts.last_mut() {
                        *c += 1;
                    }
                }
                continue;
            }
            stray_errors.push(format!("stray line {}", j + 1));
            continue;
        }
        under_option = false;
        if let Some(v) = answer_value(line) {
            if answer_line.is_none() {
                answer_line = Some(j);
                answer_val = Some(v);
            } else {
                answer_errors.push(format!("second answer line, line {}", j + 1));
            }
            continue;
        }
        stray_errors.push(format!("stray line {}", j + 1));
    }

    // A fourth option is a shape error; the minimum (two, unless the
    // heading is a skeleton) is enforced by the caller, which alone knows
    // whether the body is a skeleton.
    let mut option_shape_errors = Vec::new();
    if options.len() > 3 {
        option_shape_errors.push(format!("{} options; at most three", options.len()));
    }

    // Every option carries exactly one non-empty Why line and exactly one
    // non-empty Costs line.
    for (i, opt) in options.iter().enumerate() {
        match why_counts[i] {
            0 => option_shape_errors.push(format!("option {} missing Why", opt.letter)),
            1 => {}
            _ => option_shape_errors.push(format!("option {} has two Why lines", opt.letter)),
        }
        match costs_counts[i] {
            0 => option_shape_errors.push(format!("option {} missing Costs", opt.letter)),
            1 => {}
            _ => option_shape_errors.push(format!("option {} has two Costs lines", opt.letter)),
        }
    }

    BodyScan {
        id,
        id_count,
        supersedes,
        unrelated,
        depends,
        depends_errors,
        reference_line_errors,
        terms_errors,
        terms_line,
        options,
        answer_line,
        answer_value: answer_val,
        option_errors,
        stray_errors,
        problem_count,
        standard_count,
        option_shape_errors,
        answer_errors,
    }
}

/// A heading whose body holds only blank lines, `depends:` lines, a
/// `supersedes:` line (part of the plan, per "Does approve turn the
/// supersedes line into identifiers?"), and an `id:` line (per "What does a depends line name?" and "Does a reference
/// in a topic file carry the identifier of what it names?"): needs no id
/// of its own to stay a skeleton, but `approve` gives every heading an id,
/// skeletons included,
/// so a skeleton must stay a skeleton once it has one.
/// A `depends:` line with nothing after it, indented or not.
fn is_bare_depends_line(line: &str) -> bool {
    line.trim() == "depends:"
}

fn is_skeleton_range(lines: &[String], start: usize, end: usize) -> bool {
    ((start + 1)..end).all(|j| {
        let t = lines[j].trim();
        // Same prefix `kv` requires: "depends:"/"id:" with no
        // space after the colon is a typo, not a real field, so it must not
        // be silently swallowed as skeleton -- it falls through and
        // `scan_body` reports it as a stray line instead.
        t.is_empty()
            || t.starts_with("depends: ")
            || is_bare_depends_line(t)
            || t.starts_with("supersedes: ")
            || t.starts_with("unrelated: ")
            || t.starts_with("id: ")
    })
}

fn parse_question(lines: &[String], start: usize, end: usize, headings: &[String]) -> Section {
    // Fully trimmed (not just `trim_end`): every consumer that compares or
    // hashes `heading_text` (a depends line's target text, an `assumes`
    // row's detail, `check`'s frozen-set comparison) must agree on the same
    // string regardless of extra whitespace right after `### `.
    let heading_text = lines[start]["### ".len()..].trim().to_string();
    let is_skeleton = is_skeleton_range(lines, start, end);
    let scan = scan_body(lines, start, end, headings);

    let mut parse_errors = scan.option_errors;
    if scan.id_count > 1 {
        parse_errors.push("duplicate id line".to_string());
    }
    if let Some(v) = &scan.answer_value {
        let v = v.trim();
        let valid = v == "refused"
            || (v.chars().count() == 1
                && scan
                    .options
                    .iter()
                    .any(|o| o.letter == v.chars().next().unwrap()));
        if !valid {
            parse_errors.push(format!("answer names missing option {}", v));
        }
    }

    let mut shape_errors: Vec<String> = Vec::new();
    let mut add = |errs: Vec<String>| shape_errors.extend(errs);
    add(scan.stray_errors);
    add(scan.depends_errors);
    add(scan.reference_line_errors);
    add(scan.terms_errors);
    if !is_skeleton {
        // A missing field (count 0) and a repeated one (count > 1) are
        // different mistakes and get different messages: a zero count is
        // the field never being written, a count over one is the same
        // field written twice (178a232e A point 4).
        match scan.problem_count {
            0 => add(vec!["missing Problem line".to_string()]),
            1 => {}
            _ => add(vec!["duplicate Problem line".to_string()]),
        }
        match scan.standard_count {
            0 => add(vec!["missing Standard line".to_string()]),
            1 => {}
            _ => add(vec!["duplicate Standard line".to_string()]),
        }
        // A skeleton heading (empty body) is exempt; a real body needs two
        // or three options.
        if scan.options.len() < 2 {
            add(vec![format!("{} options; at least two", scan.options.len())]);
        }
    }
    add(scan.option_shape_errors);
    add(scan.answer_errors);
    // 178a232e A's "the answer line comes last": any non-blank line after
    // it (Problem/Standard/an option/anything else) means it isn't.
    if let Some(al) = scan.answer_line {
        if (al + 1..end).any(|j| !lines[j].trim().is_empty()) {
            add(vec![format!("answer line not last, line {}", al + 1)]);
        }
    }

    Section {
        start,
        end,
        heading_text,
        id: scan.id,
        supersedes: scan.supersedes,
        unrelated: scan.unrelated,
        depends: scan.depends,
        options: scan.options,
        answer_line: scan.answer_line,
        answer_value: scan.answer_value,
        parse_errors,
        shape_errors,
        is_skeleton,
        terms_line: scan.terms_line,
    }
}

/// Computes topic, context range, and file-level shape errors, given the
/// lines and the already heading-parsed `sections`.
fn analyze_shape(lines: &[String], sections: &[Section]) -> (Option<String>, (usize, usize), Vec<String>) {
    let mut shape_errors = Vec::new();

    let h1_indices: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| is_h1(l))
        .map(|(i, _)| i)
        .collect();
    let first_non_blank = lines.iter().position(|l| !l.trim().is_empty());

    let topic;
    let h1_idx: Option<usize>;
    if h1_indices.is_empty() {
        shape_errors.push("missing h1".to_string());
        topic = None;
        h1_idx = None;
    } else {
        if h1_indices.len() > 1 {
            for &idx in &h1_indices[1..] {
                shape_errors.push(format!("duplicate h1, line {}", idx + 1));
            }
        }
        let first_h1 = h1_indices[0];
        if let Some(fnb) = first_non_blank {
            if fnb != first_h1 {
                shape_errors.push(format!("text before h1, line {}", fnb + 1));
            }
        }
        topic = Some(lines[first_h1]["# ".len()..].trim_end().to_string());
        h1_idx = Some(first_h1);
    }

    // Per "Does the tool derive tiers from the depends lines?": tiers are
    // derived from `depends:` lines, never written by
    // hand; a `## ` line left over from the old format is a shape error.
    for (i, l) in lines.iter().enumerate() {
        if is_h2(l) {
            shape_errors.push(format!("tier heading, line {}; tiers are derived", i + 1));
        }
    }

    // Per "What is the shape of a topic file?" (9913568f A): a non-blank line
    // after a `## ` line and before the next question heading sits in no
    // question and is not prose, so it is reported as a stray line.
    let mut after_h2 = false;
    for (i, l) in lines.iter().enumerate() {
        if is_question_heading(l) {
            after_h2 = false;
        } else if is_h2(l) {
            after_h2 = true;
        } else if after_h2 && !l.trim().is_empty() && !is_h1(l) && !l.starts_with("### ") {
            shape_errors.push(format!("stray line {}", i + 1));
        }
    }

    let context_start = h1_idx.map(|i| i + 1).unwrap_or(0);
    let context_end = sections.first().map(|s| s.start).unwrap_or(lines.len());
    let context_range = (context_start.min(context_end), context_end);

    (topic, context_range, shape_errors)
}

/// sha256 of one question's own text (per "Does approve hash each question
/// on its own?"): its heading line, its `depends:` line with the answer sets
/// as written, its `supersedes:` line, an `unrelated:` line, and its body
/// (Problem, Standard, options, Why, Costs), each line trimmed at its end,
/// joined with `\n`. The `id:` line is part of it (swapping ids would re-point every
/// depends entry naming them); the lines the tool writes after that (`terms:`,
/// the `**Answer:**` line) and blank lines are left out, so asking or answering
/// a question never moves its hash (approve has already written the id). Read on the text as it stands, so call
/// it on the file after approve turned names into ids.
pub fn question_hash(rf: &RecordFile, sec: &Section) -> String {
    let (s, e) = hash_range(&rf.lines, sec);
    let text = rf.lines[s..e]
        .iter()
        .map(|l| l.trim_end())
        .filter(|l| {
            !l.trim().is_empty()
                && !l.starts_with("terms: ")
                && !l.starts_with("**Answer:**")
        })
        .collect::<Vec<_>>()
        .join("\n");
    sha256_hex(&text)
}

/// Every question's `question_hash`, in file order.
pub fn question_hashes(rf: &RecordFile) -> Vec<String> {
    rf.sections.iter().map(|s| question_hash(rf, s)).collect()
}

/// sha256 of a question's heading text alone: what the planning steps key
/// the reader's rows of a heading by ("What does the agent do to plan a
/// topic file?").
pub fn heading_hash(sec: &Section) -> String {
    sha256_hex(&sec.heading_text)
}

/// A question's heading and its body, as the reader is given them: the
/// heading text, then the Problem, Standard, options, Why and Costs lines,
/// each trimmed at its end, joined with `\n`. The `depends:`,
/// `supersedes:`, `unrelated:`, `id:` and `terms:` lines, the `**Answer:**`
/// line and blank lines are left out, so a line the agent writes after the
/// reader ran never moves it.
pub fn body_text(rf: &RecordFile, sec: &Section) -> String {
    let (s, e) = hash_range(&rf.lines, sec);
    let mut parts: Vec<&str> = vec![sec.heading_text.as_str()];
    for l in &rf.lines[(s + 1).min(e)..e] {
        let t = l.trim_end();
        if t.trim().is_empty()
            || t.starts_with("depends: ")
            || t.trim() == "depends:"
            || t.starts_with("supersedes: ")
            || t.starts_with("unrelated: ")
            || t.starts_with("id: ")
            || t.starts_with("terms: ")
            || t.starts_with("**Answer:**")
        {
            continue;
        }
        parts.push(t);
    }
    parts.join("\n")
}

/// sha256 of `body_text`.
pub fn body_hash(rf: &RecordFile, sec: &Section) -> String {
    sha256_hex(&body_text(rf, sec))
}

/// The file's hash derived from its parts (per "Does approve hash each
/// question on its own?"): sha256 of the question hashes sorted and joined
/// with `\n`. The order of the questions carries no meaning, so
/// reordering headings moves nothing.
pub fn derive_file_hash(question_hashes: &[String]) -> String {
    let mut sorted: Vec<&str> = question_hashes.iter().map(|h| h.as_str()).collect();
    sorted.sort_unstable();
    sha256_hex(&sorted.join("\n"))
}

/// The file's derived hash over its current questions.
pub fn plan_hash(rf: &RecordFile) -> String {
    derive_file_hash(&question_hashes(rf))
}

/// sha256 of the context prose lines with leading/trailing blank lines
/// trimmed, joined with `\n`; the empty string hashes when there is no
/// prose.
pub fn context_hash(rf: &RecordFile) -> String {
    let (s, e) = rf.context_range;
    let e = e.min(rf.lines.len());
    let s = s.min(e);
    let slice = &rf.lines[s..e];

    let mut from = 0;
    while from < slice.len() && slice[from].trim().is_empty() {
        from += 1;
    }
    let mut to = slice.len();
    while to > from && slice[to - 1].trim().is_empty() {
        to -= 1;
    }
    let text = slice[from..to]
        .iter()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n");
    sha256_hex(&text)
}

/// Returns [start, hash_end) — the line range whose joined bytes are hashed.
pub fn hash_range(lines: &[String], sec: &Section) -> (usize, usize) {
    // The boundary to search backward from for the last non-blank line:
    // just before the Answer line when there is one, otherwise the end of
    // the section. Trailing blank lines are never hashed, so inserting the
    // "blank line before the Answer line" (as `adr answer` does) can never
    // change a section's hash out from under itself.
    let boundary = sec.answer_line.unwrap_or(sec.end);
    let mut last = sec.start;
    for i in (sec.start..boundary).rev() {
        if !lines[i].trim().is_empty() {
            last = i;
            break;
        }
    }
    (sec.start, last + 1)
}

/// The set on a `depends:` entry, canonicalized for hashing and printing:
/// `*` for `Any`, or its letters sorted and joined with `|` for `Letters`
/// (so `=B|A` and `=A|B` hash and print the same). The one place both
/// `ask` (writing an `assumes` row) and `check` (recomputing it to compare)
/// turn a set into text.
pub fn canonical_set(set: &AnswerSet) -> String {
    match set {
        AnswerSet::Any => "*".to_string(),
        AnswerSet::Letters(letters) => {
            let mut sorted = letters.clone();
            sorted.sort_unstable();
            sorted.into_iter().map(|c| c.to_string()).collect::<Vec<_>>().join("|")
        }
    }
}

/// The `=<set>` suffix a `depends:` entry prints after its id/text: `=*`,
/// `=A`, `=A|B` (`canonical_set`, prefixed with `=`), or an empty string
/// for `None`. Shared by every place that rewrites or prints a depends
/// entry's answer set back into text.
pub fn set_suffix(set: &Option<AnswerSet>) -> String {
    set.as_ref().map(|s| format!("={}", canonical_set(s))).unwrap_or_default()
}

/// sha256 of `<dependent key>=<canonical set>` (per "Does a depends entry
/// carry the answers it holds under?", updated by "Does a reference in a
/// topic file carry the identifier of what it names?"): an `assumes`
/// row's hash, and what `check`'s frozen-set comparison
/// recomputes against the depends line as currently written. `dependent_key`
/// is the dependent section's own heading id (`assumes` rows
/// key on heading ids, not text).
pub fn assumes_hash(dependent_key: &str, set: &AnswerSet) -> String {
    sha256_hex(&format!("{}={}", dependent_key, canonical_set(set)))
}

pub fn section_hash(lines: &[String], sec: &Section) -> String {
    let (s, e) = hash_range(lines, sec);
    let text = lines[s..e]
        .iter()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n");
    sha256_hex(&text)
}

/// How many hex characters of `section_hash` the tool prints and checks as
/// a question's "version" (per "Does an answer carry the version of the
/// question the owner read?"): short enough to type on the `adr answer
/// <id>@<version>` command, long enough that two different texts for the
/// same question never collide by chance.
pub const VERSION_LEN: usize = 8;

/// The first `VERSION_LEN` hex characters of `section_hash(lines, sec)`:
/// what `show` and the page print next to an asked question, and what
/// `answer`/the page's POST handler check the owner's `<id>@<version>`
/// against.
pub fn version_of(lines: &[String], sec: &Section) -> String {
    section_hash(lines, sec)[..VERSION_LEN].to_string()
}

/// For each option in `sec`, its letter plus the `  Why: ` and `  Costs: `
/// lines immediately following it (up to the next non-blank, non-matching
/// line — typically the next option, the answer line, or the section end).
pub fn option_details(lines: &[String], sec: &Section) -> Vec<(char, Option<String>, Option<String>)> {
    let mut out = Vec::new();
    for opt in &sec.options {
        let mut why = None;
        let mut costs = None;
        for line in &lines[(opt.line + 1).min(lines.len())..sec.end.min(lines.len())] {
            if let Some((kind, v)) = detail_line(line) {
                match kind {
                    Detail::Why if why.is_none() => why = Some(v.trim().to_string()),
                    Detail::Costs if costs.is_none() => costs = Some(v.trim().to_string()),
                    _ => {}
                }
                continue;
            }
            if line.trim().is_empty() {
                continue;
            }
            break;
        }
        out.push((opt.letter, why, costs));
    }
    out
}

/// The cap on how many live questions one derived tier may hold. A different
/// value is a new question and a new release; no owner command sets it.
pub const TIER_CAP: usize = 4;

/// Assigns every section a 1-based tier: a section with no heading
/// `depends:` entry is tier 1 (a root); otherwise its tier is one more than
/// the highest tier among the sections its heading entries name. Term and
/// decision entries never affect tier (their resolution is checked by the
/// commands, not here).
///
/// Self-dep and dangling entries are found and reported (in file order)
/// before any tier is computed; a cycle is found by the recursive descent
/// below and reports the headings on the cycle in traversal order.
pub fn derive_tiers(rf: &RecordFile) -> Result<Vec<usize>, DeriveError> {
    let n = rf.sections.len();

    let mut id_idx: HashMap<&str, usize> = HashMap::new();
    for (i, sec) in rf.sections.iter().enumerate() {
        if let Some(id) = &sec.id {
            id_idx.entry(id.as_str()).or_insert(i);
        }
    }
    let mut heading_idx: HashMap<&str, usize> = HashMap::new();
    for (i, sec) in rf.sections.iter().enumerate() {
        heading_idx.entry(sec.heading_text.trim()).or_insert(i);
    }

    let mut deps: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, sec) in rf.sections.iter().enumerate() {
        for d in &sec.depends {
            match d {
                // An `Id` entry is a heading dep only when it matches a
                // heading id in this same file (per "Does a reference in a
                // topic file carry the identifier of what it names?");
                // otherwise it is a
                // term or decision reference, resolved elsewhere, and never
                // affects tier -- never a Dangling error either, since it
                // may legitimately resolve outside this file.
                Dep::Id { id, .. } => {
                    if let Some(&j) = id_idx.get(id.as_str()) {
                        if j == i {
                            return Err(DeriveError::SelfDep(sec.heading_text.clone()));
                        }
                        deps[i].push(j);
                    }
                }
                // A `Text` entry is a heading dep only when heading-shaped
                // (ends in `?`); this is the pre-approval form, resolved
                // against headings in this file in memory so `show` can
                // still derive tiers before `approve` has minted anything.
                Dep::Text { text, .. } => {
                    let trimmed = text.trim();
                    if !trimmed.ends_with('?') {
                        continue;
                    }
                    if trimmed == sec.heading_text.trim() {
                        return Err(DeriveError::SelfDep(sec.heading_text.clone()));
                    }
                    match heading_idx.get(trimmed) {
                        Some(&j) => deps[i].push(j),
                        None => {
                            return Err(DeriveError::Dangling {
                                section: sec.heading_text.clone(),
                                entry: text.clone(),
                            })
                        }
                    }
                }
            }
        }
    }

    let mut tier: Vec<Option<usize>> = vec![None; n];
    let mut state: Vec<u8> = vec![0; n];
    let mut path: Vec<usize> = Vec::new();
    for i in 0..n {
        if state[i] == 0 {
            derive_visit(i, rf, &deps, &mut tier, &mut state, &mut path)?;
        }
    }
    Ok(tier.into_iter().map(|t| t.unwrap_or(1)).collect())
}

fn derive_visit(
    i: usize,
    rf: &RecordFile,
    deps: &[Vec<usize>],
    tier: &mut [Option<usize>],
    state: &mut [u8],
    path: &mut Vec<usize>,
) -> Result<usize, DeriveError> {
    if let Some(t) = tier[i] {
        return Ok(t);
    }
    if state[i] == 1 {
        let pos = path.iter().position(|&x| x == i).expect("i is on the path");
        let cycle_headings: Vec<String> = path[pos..]
            .iter()
            .map(|&x| rf.sections[x].heading_text.clone())
            .collect();
        return Err(DeriveError::Cycle(cycle_headings));
    }
    state[i] = 1;
    path.push(i);
    let mut max_dep_tier = 0usize;
    for &j in &deps[i] {
        let t = derive_visit(j, rf, deps, tier, state, path)?;
        if t > max_dep_tier {
            max_dep_tier = t;
        }
    }
    path.pop();
    state[i] = 2;
    let t = if deps[i].is_empty() { 1 } else { max_dep_tier + 1 };
    tier[i] = Some(t);
    Ok(t)
}

/// `(tier, count)` for every tier that has at least one section, sorted by
/// tier.
pub fn tier_sizes(tiers: &[usize]) -> Vec<(usize, usize)> {
    let mut counts: std::collections::BTreeMap<usize, usize> = std::collections::BTreeMap::new();
    for &t in tiers {
        *counts.entry(t).or_insert(0) += 1;
    }
    counts.into_iter().collect()
}

/// The first (lowest-numbered) tier holding more than `TIER_CAP` sections,
/// if any (per "Does the tool cap the questions in a tier?").
pub fn over_cap(tiers: &[usize]) -> Option<(usize, usize)> {
    tier_sizes(tiers).into_iter().find(|&(_, count)| count > TIER_CAP)
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn sha256_hex(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hex(&hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rf_from(content: &str) -> RecordFile {
        let lines = split_lines(content);
        build(PathBuf::from("test.md"), "test.md".to_string(), lines)
    }

    const SHAPED: &str = "# Topic\n\ncontext prose line one.\ncontext prose line two.\n\n### 1.1 First question?\n\nProblem: p.\n\nStandard: s.\n\n- A) Do the thing.\n  Why: reasons.\n  Costs: costs.\n- B) Do the other thing.\n  Why: other reasons.\n  Costs: other costs.\n";

    #[test]
    fn h1_required_and_first() {
        let rf = rf_from("no h1 here\n\n### 1.1 Q?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n");
        assert!(rf.topic.is_none());
        assert!(rf.shape_errors.iter().any(|e| e.contains("missing h1")));

        let rf2 = rf_from("stray text\n# Topic\n");
        assert_eq!(rf2.topic.as_deref(), Some("Topic"));
        assert!(rf2.shape_errors.iter().any(|e| e.contains("text before h1")));

        let rf3 = rf_from("# Topic\n\n# Second Topic\n");
        assert!(rf3.shape_errors.iter().any(|e| e.contains("duplicate h1")));

        let rf4 = rf_from(SHAPED);
        assert_eq!(rf4.topic.as_deref(), Some("Topic"));
        assert!(rf4.shape_errors.is_empty(), "{:?}", rf4.shape_errors);
    }

    #[test]
    fn a_h2_line_is_a_tier_heading_shape_error() {
        let rf = rf_from("# Topic\n\n## Old Group\n\n### 1.1 Q?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n");
        assert!(
            rf.shape_errors
                .iter()
                .any(|e| e.starts_with("tier heading, line 3")),
            "{:?}",
            rf.shape_errors
        );
    }

    #[test]
    fn context_hash_stable_when_a_question_body_changes() {
        let rf1 = rf_from(SHAPED);
        let h1 = context_hash(&rf1);

        let edited = SHAPED.replace("Problem: p.", "Problem: a totally different problem.");
        let rf2 = rf_from(&edited);
        let h2 = context_hash(&rf2);

        assert_eq!(h1, h2, "context hash must not move when only a body changes");
        assert_ne!(h1, sha256_hex(""), "sanity: this record does have context prose");
    }

    #[test]
    fn skeleton_heading_has_no_id_and_is_skeleton() {
        let rf = rf_from("# Topic\n\n### 1.1 Skeleton?\n\n### 1.2 Real question?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n");
        let skeleton = &rf.sections[0];
        assert!(skeleton.is_skeleton);
        assert!(skeleton.id.is_none());
        assert!(skeleton.shape_errors.is_empty(), "{:?}", skeleton.shape_errors);

        let real = &rf.sections[1];
        assert!(!real.is_skeleton);
    }

    #[test]
    fn skeleton_with_only_depends_lines_is_skeleton() {
        let rf = rf_from("# Topic\n\n### 1.1 Skeleton?\ndepends: widget, 1.2 Other?\n\n### 1.2 Other?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n");
        let sk = &rf.sections[0];
        assert!(sk.is_skeleton);
        assert!(sk.id.is_none());
        assert_eq!(sk.depends.len(), 2);
        assert!(sk.shape_errors.is_empty(), "{:?}", sk.shape_errors);
    }

    #[test]
    fn depends_typo_missing_the_space_is_not_a_skeleton_line() {
        // `kv` only recognizes "depends: " (with the space); a
        // heading whose only body line is the typo "depends:widget" (no
        // space) must not be silently accepted as a skeleton -- it falls
        // through `scan_body` and is reported as a stray line instead, so
        // the owner sees the typo rather than the line vanishing.
        let content = "# Topic\n\n### 1.1 Q?\ndepends:widget\n";
        let rf = rf_from(content);
        let sec = &rf.sections[0];
        assert!(!sec.is_skeleton, "{:?}", sec);
        assert!(
            sec.shape_errors.iter().any(|e| e == "stray line 4"),
            "{:?}",
            sec.shape_errors
        );
    }

    #[test]
    fn stray_line_in_body_is_a_shape_error_naming_the_line() {
        let content = "# Topic\n\n### 1.1 Q?\n\nProblem: p.\n\nStandard: s.\n\nthis line is not allowed\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
        let rf = rf_from(content);
        let sec = &rf.sections[0];
        let stray_line_no = content
            .lines()
            .position(|l| l == "this line is not allowed")
            .unwrap()
            + 1;
        assert!(
            sec.shape_errors
                .iter()
                .any(|e| e == &format!("stray line {}", stray_line_no)),
            "{:?}",
            sec.shape_errors
        );
    }

    //
    // Decision A's Problem/Standard presence clause, plus (via `SHAPED`
    // below) a well-formed body -- one-line Problem, one-line Standard,
    // two options each with a Why and a Costs line -- parsing with zero
    // shape errors.
    #[test]
    fn missing_problem_or_standard_only_fails_non_skeleton_questions() {
        let rf = rf_from("# Topic\n\n### 1.1 Skeleton?\n\n### 1.2 Has a body but no Problem/Standard?\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n");
        let skeleton = &rf.sections[0];
        assert!(skeleton.shape_errors.is_empty());

        let non_skeleton = &rf.sections[1];
        assert!(non_skeleton
            .shape_errors
            .iter()
            .any(|e| e == "missing Problem line"));
        assert!(non_skeleton
            .shape_errors
            .iter()
            .any(|e| e == "missing Standard line"));

        let rf2 = rf_from(SHAPED);
        assert!(rf2.sections[0].shape_errors.is_empty(), "{:?}", rf2.sections[0].shape_errors);
    }

    #[test]
    fn plan_hash_changes_on_heading_add_rename_and_body_edit_not_on_reorder() {
        let base = rf_from(SHAPED);
        let base_hash = plan_hash(&base);

        // Body edit: moves the hash (the body is part of the question).
        let body_edited = SHAPED.replace("Problem: p.", "Problem: a very different problem.");
        assert_ne!(base_hash, plan_hash(&rf_from(&body_edited)), "a body edit must move the hash");

        // Heading added.
        let added = format!("{}\n### 1.2 A second question?\n\nProblem: p2.\n\nStandard: s2.\n\n- A) y.\n  Why: w.\n  Costs: c.\n- B) z.\n  Why: w.\n  Costs: c.\n", SHAPED);
        assert_ne!(base_hash, plan_hash(&rf_from(&added)), "adding a heading must move the hash");

        // Heading renamed.
        let renamed = SHAPED.replace("### 1.1 First question?", "### 1.1 First question, renamed?");
        assert_ne!(base_hash, plan_hash(&rf_from(&renamed)), "renaming a heading must move the hash");

        // Headings reordered: two questions, swapped.
        let two_questions = "# Topic\n\n### 1.1 Question A?\n\nProblem: pa.\n\nStandard: sa.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) x2.\n  Why: w.\n  Costs: c.\n\n### 1.2 Question B?\n\nProblem: pb.\n\nStandard: sb.\n\n- A) y.\n  Why: w.\n  Costs: c.\n- B) y2.\n  Why: w.\n  Costs: c.\n";
        let swapped = "# Topic\n\n### 1.2 Question B?\n\nProblem: pb.\n\nStandard: sb.\n\n- A) y.\n  Why: w.\n  Costs: c.\n- B) y2.\n  Why: w.\n  Costs: c.\n\n### 1.1 Question A?\n\nProblem: pa.\n\nStandard: sa.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) x2.\n  Why: w.\n  Costs: c.\n";
        assert_eq!(
            plan_hash(&rf_from(two_questions)),
            plan_hash(&rf_from(swapped)),
            "reordering headings must not move the derived hash"
        );
    }

    #[test]
    fn question_hash_covers_depends_with_sets_and_skips_tool_lines() {
        let make = |set_suffix: &str| {
            format!(
                "# Topic\n\n### 1.1 Q?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n### 1.2 R?\ndepends: 1.1 Q?{}\n\nProblem: p2.\n\nStandard: s2.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
                set_suffix
            )
        };
        let with_a = rf_from(&make("=A"));
        let with_b = rf_from(&make("=B"));
        assert_ne!(
            question_hashes(&with_a)[1],
            question_hashes(&with_b)[1],
            "the answer set on a depends entry is part of the question hash"
        );
        assert_eq!(question_hashes(&with_a)[0], question_hashes(&with_b)[0]);

        let base = "# Topic\n\n### 1.1 Q?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
        let tooled = "# Topic\n\n### 1.1 Q?\nterms: a=b\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n**Answer:** A\n";
        assert_eq!(
            question_hashes(&rf_from(base)),
            question_hashes(&rf_from(tooled)),
            "terms and Answer lines are not part of the question hash"
        );
    }

    #[test]
    fn section_hash_ignores_trailing_whitespace_on_a_line() {
        let clean = rf_from(SHAPED);
        let clean_hash = section_hash(&clean.lines, &clean.sections[0]);

        let trailing = SHAPED.replace("- A) Do the thing.", "- A) Do the thing.  ");
        let dirty = rf_from(&trailing);
        let dirty_hash = section_hash(&dirty.lines, &dirty.sections[0]);

        assert_eq!(
            clean_hash, dirty_hash,
            "trailing whitespace on an option line must not move the section hash"
        );
    }

    // the supersedes list, like
    // depends, holds several entries as one line of comma-separated values,
    // not one entry per line.
    #[test]
    fn supersedes_line_parses_multiple_comma_separated_ids() {
        let rf = rf_from("# Topic\n\n### 1.1 Q?\n\nid: new-id\n\nsupersedes: old-1, old-2\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n");
        assert_eq!(
            rf.sections[0].supersedes,
            vec!["old-1".to_string(), "old-2".to_string()]
        );
    }

    //
    // Decision A's "each one sentence" clause, at the unit that decides
    // *which* sentence: text with a sentence terminator yields that first
    // sentence -- still the sentence recorded on the option regardless of
    // what follows it, since `chosen_sentence` should show the same text
    // whether or not the option is later refused for holding a second one
    // (that refusal is `scan_body`'s separate `terms::sentence_count` check,
    // covered by `an_option_with_more_than_one_sentence_is_a_parse_error`
    // below). Text with no terminator at all yields `None`, which
    // `scan_body` turns into "option X has no decision sentence".
    #[test]
    fn an_option_decides_exactly_its_first_sentence() {
        assert_eq!(
            extract_sentence("Ties live in code. Because tests move."),
            Some("Ties live in code.".to_string())
        );
        assert_eq!(extract_sentence("Ties live in code"), None);
    }

    //
    // Decision A's "each one sentence" clause, the half
    // `an_option_decides_exactly_its_first_sentence` above does not cover:
    // a decision text holding a second sentence is a parse error (a
    // `terms::sentence_count` > 1 on the option's own text), not silently
    // accepted just because the first sentence parsed fine.
    #[test]
    fn an_option_with_more_than_one_sentence_is_a_parse_error() {
        let rf = rf_from(
            "# T\n\n### Q?\n\nProblem: p.\nStandard: s.\n\n- A) One. And a second sentence.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n",
        );
        assert!(
            rf.sections[0]
                .parse_errors
                .iter()
                .any(|e| e == "option A decision text has 2 sentences, more than one"),
            "{:?}",
            rf.sections[0].parse_errors
        );

        let ok = rf_from(SHAPED);
        assert!(ok.sections[0].parse_errors.is_empty(), "{:?}", ok.sections[0].parse_errors);
    }

    //
    // Decision A's "a second Problem:/Standard: line" fix (point 4): a
    // repeated field is reported as a duplicate, not folded into the same
    // "missing" message a zero count gets.
    #[test]
    fn a_repeated_problem_or_standard_line_is_a_duplicate_not_a_missing_one() {
        let rf = rf_from(
            "# T\n\n### Q?\n\nProblem: p.\n\nProblem: p again.\n\nStandard: s.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n",
        );
        assert!(
            rf.sections[0].shape_errors.iter().any(|e| e == "duplicate Problem line"),
            "{:?}",
            rf.sections[0].shape_errors
        );
        assert!(
            !rf.sections[0].shape_errors.iter().any(|e| e == "missing Problem line"),
            "{:?}",
            rf.sections[0].shape_errors
        );

        let rf2 = rf_from(
            "# T\n\n### Q?\n\nProblem: p.\n\nStandard: s.\n\nStandard: s again.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n",
        );
        assert!(
            rf2.sections[0].shape_errors.iter().any(|e| e == "duplicate Standard line"),
            "{:?}",
            rf2.sections[0].shape_errors
        );
    }

    //
    // Decision A's "the answer line comes last" clause (point 3): an
    // `**Answer:**` line with real body content after it is refused, at
    // the section-parsing level `ask`/`check` both read.
    #[test]
    fn an_answer_line_before_the_rest_of_the_body_is_a_shape_error() {
        let rf = rf_from(
            "# T\n\n### Q?\n\n**Answer:** A\n\nProblem: p.\nStandard: s.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n",
        );
        assert!(
            rf.sections[0].shape_errors.iter().any(|e| e.starts_with("answer line not last")),
            "{:?}",
            rf.sections[0].shape_errors
        );

        // At the true end (nothing but blank lines after it): accepted.
        let ok = rf_from(
            "# T\n\n### Q?\n\nProblem: p.\nStandard: s.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n\n**Answer:** A\n",
        );
        assert!(
            !ok.sections[0].shape_errors.iter().any(|e| e.starts_with("answer line not last")),
            "{:?}",
            ok.sections[0].shape_errors
        );
    }

    //
    // Decision A's "the answer line comes last" clause: a second
    // `**Answer:**` line is its own shape error, not silently swallowed by
    // `answer_line` being overwritten to point at the later (last) one --
    // which used to let an out-of-place early answer line escape the
    // "answer line not last" check entirely.
    #[test]
    fn a_second_answer_line_is_a_shape_error() {
        let rf = rf_from(
            "# T\n\n### Q?\n\nProblem: p.\nStandard: s.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n\n**Answer:** A\n\n**Answer:** B\n",
        );
        assert!(
            rf.sections[0].shape_errors.iter().any(|e| e.starts_with("second answer line")),
            "{:?}",
            rf.sections[0].shape_errors
        );
    }

    #[test]
    fn a_lean_line_inside_a_question_is_a_shape_error() {
        let rf = rf_from(
            "# T\n\n### Q?\n\nProblem: p.\nStandard: s.\nLean: A.\n\n- A) Yes.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n",
        );
        assert!(rf.sections[0]
            .shape_errors
            .iter()
            .any(|e| e.starts_with("stray line")));
    }

    #[test]
    fn a_why_line_before_any_option_is_stray() {
        let rf = rf_from(
            "# T\n\n### Q?\n\nProblem: p.\nStandard: s.\n  Why: early.\n\n- A) Yes.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n",
        );
        assert!(rf.sections[0]
            .shape_errors
            .iter()
            .any(|e| e == "stray line 7"));
        let rf = rf_from(
            "# T\n\n### Q?\n\nProblem: p.\nStandard: s.\n\n- A) Yes.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n\n**Answer:** A\n  Costs: late.\n",
        );
        assert!(rf.sections[0]
            .shape_errors
            .iter()
            .any(|e| e.starts_with("stray line")));
    }

    //
    // Decision A's "two or three lettered options" clause: four is
    // refused, three is accepted (the `ok` case below).
    #[test]
    fn a_fourth_option_is_a_shape_error() {
        let rf = rf_from(
            "# T\n\n### Q?\n\nProblem: p.\nStandard: s.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n- C) Three.\n  Why: w.\n  Costs: c.\n- D) Four.\n  Why: w.\n  Costs: c.\n",
        );
        assert!(rf.sections[0]
            .shape_errors
            .iter()
            .any(|e| e == "4 options; at most three"));
        let ok = rf_from(
            "# T\n\n### Q?\n\nProblem: p.\nStandard: s.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n- C) Three.\n  Why: w.\n  Costs: c.\n",
        );
        assert!(ok.sections[0].shape_errors.is_empty());
    }

    #[test]
    fn one_option_is_a_shape_error() {
        let rf = rf_from(
            "# T\n\n### Q?\n\nProblem: p.\nStandard: s.\n\n- A) Only choice.\n  Why: w.\n  Costs: c.\n",
        );
        assert!(rf.sections[0]
            .shape_errors
            .iter()
            .any(|e| e == "1 options; at least two"));
        assert!(rf.sections[0].parse_errors.is_empty(), "{:?}", rf.sections[0].parse_errors);
    }

    #[test]
    fn answer_refused_is_recognized() {
        let content = "# Topic\n\n### 1.1 Q?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n**Answer:** refused\n";
        let rf = rf_from(content);
        let sec = &rf.sections[0];
        assert!(sec.is_refused());
        assert_eq!(sec.answer_letter(), None);
        assert!(sec.parse_errors.is_empty(), "{:?}", sec.parse_errors);
    }

    // a one-line, comma-
    // separated depends list with three entries parses cleanly (no shape
    // errors) and each entry keeps its own answer set independently: the
    // middle entry carries `=A`, the other two carry none.
    #[test]
    fn depends_parses_all_three_entry_forms_and_sets() {
        let rf = rf_from(
            "# Topic\n\n\
             ### 1.1 First question?\n\n\
             Problem: p.\n\n\
             Standard: s.\n\n\
             - A) x.\n  Why: w.\n  Costs: c.\n\
             - B) y.\n  Why: w.\n  Costs: c.\n\n\
             ### 1.2 Second question?\n\
             depends: widget, 1.1 First question?=A, 11111111-1111-1111-1111-111111111111\n\n\
             Problem: p2.\n\n\
             Standard: s2.\n\n\
             - A) x.\n  Why: w.\n  Costs: c.\n\
             - B) y.\n  Why: w.\n  Costs: c.\n",
        );
        let sec = &rf.sections[1];
        assert_eq!(sec.shape_errors, Vec::<String>::new(), "{:?}", sec.shape_errors);
        assert_eq!(sec.depends.len(), 3);
        assert_eq!(
            sec.depends[0],
            Dep::Text {
                text: "widget".to_string(),
                set: None
            }
        );
        assert_eq!(
            sec.depends[1],
            Dep::Text {
                text: "1.1 First question?".to_string(),
                set: Some(AnswerSet::Letters(vec!['A']))
            }
        );
        assert_eq!(
            sec.depends[2],
            Dep::Id {
                id: "11111111-1111-1111-1111-111111111111".to_string(),
                set: None
            }
        );
    }

    #[test]
    fn depends_set_suffixes_parse_letters_and_star() {
        let rf = rf_from(
            "# Topic\n\n\
             ### 1.1 Q?\n\n\
             Problem: p.\n\n\
             Standard: s.\n\n\
             - A) x.\n  Why: w.\n  Costs: c.\n\
             - B) y.\n  Why: w.\n  Costs: c.\n\n\
             ### 1.2 R?\n\
             depends: 1.1 Q?=A|B\n\n\
             Problem: p2.\n\n\
             Standard: s2.\n\n\
             - A) x.\n  Why: w.\n  Costs: c.\n\
             - B) y.\n  Why: w.\n  Costs: c.\n\n\
             ### 1.3 S?\n\
             depends: 1.1 Q?=*\n\n\
             Problem: p3.\n\n\
             Standard: s3.\n\n\
             - A) x.\n  Why: w.\n  Costs: c.\n\
             - B) y.\n  Why: w.\n  Costs: c.\n",
        );
        assert_eq!(
            rf.sections[1].depends[0],
            Dep::Text {
                text: "1.1 Q?".to_string(),
                set: Some(AnswerSet::Letters(vec!['A', 'B']))
            }
        );
        assert_eq!(
            rf.sections[2].depends[0],
            Dep::Text {
                text: "1.1 Q?".to_string(),
                set: Some(AnswerSet::Any)
            }
        );
    }

    // two heading-shaped
    // entries on the *same* depends line, each with its own distinct
    // answer set: the sets must not bleed into each other or collapse to
    // one set for the whole line.
    #[test]
    fn depends_two_entries_on_one_line_keep_separate_answer_sets() {
        let rf = rf_from(
            "# Topic\n\n\
             ### 1.1 Q?\n\n\
             Problem: p.\n\n\
             Standard: s.\n\n\
             - A) x.\n  Why: w.\n  Costs: c.\n\
             - B) y.\n  Why: w.\n  Costs: c.\n\n\
             ### 1.2 Q2?\n\n\
             Problem: p.\n\n\
             Standard: s.\n\n\
             - A) x.\n  Why: w.\n  Costs: c.\n\
             - B) y.\n  Why: w.\n  Costs: c.\n\n\
             ### 1.3 R?\n\
             depends: 1.1 Q?=A, 1.2 Q2?=A|B\n\n\
             Problem: p3.\n\n\
             Standard: s3.\n\n\
             - A) x.\n  Why: w.\n  Costs: c.\n\
             - B) y.\n  Why: w.\n  Costs: c.\n",
        );
        let sec = &rf.sections[2];
        assert_eq!(sec.shape_errors, Vec::<String>::new(), "{:?}", sec.shape_errors);
        assert_eq!(
            sec.depends[0],
            Dep::Text {
                text: "1.1 Q?".to_string(),
                set: Some(AnswerSet::Letters(vec!['A']))
            },
            "the first entry's set must be its own, not the second entry's"
        );
        assert_eq!(
            sec.depends[1],
            Dep::Text {
                text: "1.2 Q2?".to_string(),
                set: Some(AnswerSet::Letters(vec!['A', 'B']))
            },
            "the second entry's set must be its own, not the first entry's"
        );
    }

    #[test]
    fn depends_malformed_entries_on_one_line_are_shape_errors() {
        // All four problems (an empty entry, a bad set, and a duplicate) are
        // entries on the same, single `depends:` line -- per "Does a
        // reference line hold its whole list on one line?": a section
        // holds at most one `depends:` line, comma separated -- so every
        // error below names the same line number.
        let rf = rf_from(
            "# Topic\n\n\
             ### 1.1 Q?\n\
             depends: , 1.1 Q?=a, widget, widget\n\n\
             Problem: p.\n\n\
             Standard: s.\n\n\
             - A) x.\n  Why: w.\n  Costs: c.\n\
             - B) y.\n  Why: w.\n  Costs: c.\n",
        );
        let sec = &rf.sections[0];
        assert!(
            sec.shape_errors.iter().any(|e| e == "depends: empty entry, line 4"),
            "{:?}",
            sec.shape_errors
        );
        assert!(
            sec.shape_errors
                .iter()
                .any(|e| e == "depends: bad set syntax, line 4"),
            "{:?}",
            sec.shape_errors
        );
        assert!(
            sec.shape_errors
                .iter()
                .any(|e| e == "depends: duplicate entry, line 4"),
            "{:?}",
            sec.shape_errors
        );
    }

    // a depends list split
    // across two lines is refused (a shape error), not read as more
    // entries of the same list.
    #[test]
    fn two_depends_lines_under_one_heading_is_a_shape_error() {
        let rf = rf_from(
            "# Topic\n\n### 1.1 Q?\ndepends: widget\ndepends: gadget\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        );
        let sec = &rf.sections[0];
        // Only the first line's entry is parsed; the second line is refused
        // outright rather than treated as more entries.
        assert_eq!(
            sec.depends,
            vec![Dep::Text {
                text: "widget".to_string(),
                set: None
            }]
        );
        assert!(
            sec.shape_errors
                .iter()
                .any(|e| e == "depends: second depends line, line 5"),
            "{:?}",
            sec.shape_errors
        );
    }

    #[test]
    fn bare_depends_predicate_ignores_indent() {
        assert!(is_bare_depends_line("depends:"));
        assert!(is_bare_depends_line("  depends:  "));
        assert!(!is_bare_depends_line("depends: x"));
        assert!(!is_bare_depends_line("depends:x"));
    }

    #[test]
    fn indented_bare_depends_is_the_same_as_bare_and_a_later_real_one_is_second() {
        for bare in ["depends:", "  depends:"] {
            let rf = rf_from(&format!("# T\n\n### Q?\n{bare}\ndepends: widget\n"));
            let sec = &rf.sections[0];
            assert!(
                sec.shape_errors.iter().any(|e| e.starts_with("depends: second depends line")),
                "{bare:?}: {:?}",
                sec.shape_errors
            );
            assert!(
                !sec.shape_errors.iter().any(|e| e.contains("stray")),
                "{bare:?}: {:?}",
                sec.shape_errors
            );
        }
    }

    #[test]
    fn unrelated_line_parses_is_in_the_question_hash_and_not_in_the_body_hash() {
        let text = "# Topic\n\n### Q?\ndepends: widget\nunrelated: a, b\nid: new-id\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
        let rf = rf_from(text);
        assert_eq!(rf.sections[0].unrelated, vec!["a".to_string(), "b".to_string()]);
        assert!(rf.shape_errors.is_empty() && rf.sections[0].shape_errors.is_empty());
        let other = rf_from(&text.replace("unrelated: a, b", "unrelated: a, c"));
        assert_ne!(question_hash(&rf, &rf.sections[0]), question_hash(&other, &other.sections[0]));
        assert_eq!(body_hash(&rf, &rf.sections[0]), body_hash(&other, &other.sections[0]));
        assert_eq!(heading_hash(&rf.sections[0]), sha256_hex("Q?"));
        assert!(body_text(&rf, &rf.sections[0]).starts_with("Q?\nProblem: p.\nStandard: s.\n- A) x."));
        let twice = rf_from(&text.replace("id: new-id", "unrelated: c\nid: new-id"));
        assert!(twice.sections[0].shape_errors.iter().any(|e| e.contains("second unrelated line")));
    }

    // a supersedes list split
    // across two lines is refused (a shape error), not silently dropped.
    #[test]
    fn two_supersedes_lines_under_one_heading_is_a_shape_error() {
        let rf = rf_from(
            "# Topic\n\n### 1.1 Q?\ndepends: widget\nsupersedes: aaa, bbb\nsupersedes: ccc\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        );
        let sec = &rf.sections[0];
        // Only the first line's entries are read.
        assert_eq!(sec.supersedes, vec!["aaa".to_string(), "bbb".to_string()]);
        assert!(
            sec.shape_errors
                .iter()
                .any(|e| e == "supersedes: second supersedes line, line 6"),
            "{:?}",
            sec.shape_errors
        );
    }

    // a terms list split
    // across two lines is refused (a shape error), not silently dropped.
    #[test]
    fn two_terms_lines_under_one_heading_is_a_shape_error() {
        let rf = rf_from(
            "# Topic\n\n### 1.1 Q?\ndepends: widget\nterms: widget=aaa\nterms: gadget=bbb\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        );
        let sec = &rf.sections[0];
        // The first terms line stays the one the section records.
        assert_eq!(sec.terms_line, Some(4));
        assert!(
            sec.shape_errors
                .iter()
                .any(|e| e == "terms: second terms line, line 6"),
            "{:?}",
            sec.shape_errors
        );
    }

    #[test]
    fn depends_entry_with_a_comma_in_heading_text_rejoins_against_the_file_headings() {
        let rf = rf_from(
            "# Topic\n\n\
             ### First, then second?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
             ### R?\ndepends: First, then second?\n\nProblem: p2.\n\nStandard: s2.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        );
        let sec = &rf.sections[1];
        assert_eq!(sec.shape_errors, Vec::<String>::new(), "{:?}", sec.shape_errors);
        assert_eq!(sec.depends.len(), 1, "the comma must not split into two entries");
        assert_eq!(
            sec.depends[0],
            Dep::Text {
                text: "First, then second?".to_string(),
                set: None
            }
        );
        assert_eq!(rf.dep_target(&sec.depends[0]), Some(0));
    }

    #[test]
    fn depends_comma_rejoin_prefers_the_longest_heading_match() {
        // "Alpha, Beta?" is a real heading in this file too; the rejoin must
        // still prefer the longer, full match "Alpha, Beta, Gamma?" over the
        // shorter one it could stop at ("longest match wins").
        let rf = rf_from(
            "# Topic\n\n\
             ### Alpha, Beta?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
             ### Alpha, Beta, Gamma?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
             ### R?\ndepends: Alpha, Beta, Gamma?=A\n\nProblem: p2.\n\nStandard: s2.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        );
        let sec = rf.sections.iter().find(|s| s.heading_text == "R?").unwrap();
        assert_eq!(sec.depends.len(), 1, "{:?}", sec.depends);
        assert_eq!(
            sec.depends[0],
            Dep::Text {
                text: "Alpha, Beta, Gamma?".to_string(),
                set: Some(AnswerSet::Letters(vec!['A']))
            }
        );
    }

    #[test]
    fn derive_tiers_roots_and_chain() {
        let rf = rf_from(
            "# Topic\n\n\
             ### A?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
             ### B?\ndepends: A?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
             ### C?\ndepends: B?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        );
        assert_eq!(rf.tiers, Some(vec![1, 2, 3]));
        assert_eq!(rf.tier_of(0), Some(1));
        assert_eq!(rf.tier_of(2), Some(3));
        assert!(rf.derive_error.is_none());
    }

    #[test]
    fn derive_tiers_dangling_entry_errors() {
        let rf = rf_from(
            "# Topic\n\n### A?\ndepends: Missing?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        );
        assert!(rf.tiers.is_none());
        match &rf.derive_error {
            Some(DeriveError::Dangling { section, entry }) => {
                assert_eq!(section, "A?");
                assert_eq!(entry, "Missing?");
            }
            other => panic!("expected Dangling, got {:?}", other),
        }
    }

    #[test]
    fn derive_tiers_cycle_errors() {
        let rf = rf_from(
            "# Topic\n\n\
             ### A?\ndepends: B?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
             ### B?\ndepends: A?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        );
        match &rf.derive_error {
            Some(DeriveError::Cycle(names)) => {
                assert!(names.contains(&"A?".to_string()), "{:?}", names);
                assert!(names.contains(&"B?".to_string()), "{:?}", names);
            }
            other => panic!("expected Cycle, got {:?}", other),
        }
    }

    #[test]
    fn derive_tiers_self_dep_errors() {
        let rf = rf_from(
            "# Topic\n\n### A?\ndepends: A?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        );
        match &rf.derive_error {
            Some(DeriveError::SelfDep(h)) => assert_eq!(h, "A?"),
            other => panic!("expected SelfDep, got {:?}", other),
        }
    }

    #[test]
    fn over_cap_finds_the_first_tier_past_the_cap() {
        assert_eq!(TIER_CAP, 4);
        let tiers = vec![1, 1, 1, 1, 1];
        assert_eq!(over_cap(&tiers), Some((1, 5)));
        assert_eq!(tier_sizes(&tiers), vec![(1, 5)]);

        let ok = vec![1, 1, 1, 1];
        assert_eq!(over_cap(&ok), None);

        let mixed = vec![1, 1, 2, 2, 2, 2, 2];
        assert_eq!(over_cap(&mixed), Some((2, 5)));
        assert_eq!(tier_sizes(&mixed), vec![(1, 2), (2, 5)]);
    }

    #[test]
    fn dependents_of_lists_heading_deps_with_their_sets() {
        let rf = rf_from(
            "# Topic\n\n\
             ### A?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
             ### B?\ndepends: A?=A\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
             ### C?\ndepends: A?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        );
        let deps = rf.dependents_of(0);
        assert_eq!(deps.len(), 2, "{:?}", deps.iter().map(|(i, _)| i).collect::<Vec<_>>());
        assert!(deps
            .iter()
            .any(|(i, set)| *i == 1 && *set == Some(&AnswerSet::Letters(vec!['A']))));
        assert!(deps.iter().any(|(i, set)| *i == 2 && set.is_none()));
    }

    #[test]
    fn skeleton_with_an_id_line_is_still_skeleton() {
        // Per "Does a reference in a topic file carry the identifier of
        // what it names?": `approve` gives every heading an id, skeletons
        // included; a skeleton that has picked one up must not suddenly start requiring
        // Problem/Standard/options.
        let rf = rf_from(
            "# Topic\n\n### 1.1 Skeleton?\n\nid: 11111111-1111-1111-1111-111111111111\n\ndepends: widget\n\n### 1.2 Other?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        );
        let sk = &rf.sections[0];
        assert!(sk.is_skeleton, "{:?}", sk);
        assert_eq!(
            sk.id.as_deref(),
            Some("11111111-1111-1111-1111-111111111111")
        );
        assert!(sk.shape_errors.is_empty(), "{:?}", sk.shape_errors);
    }

    #[test]
    fn question_hash_changes_when_an_id_line_changes() {
        let base = "# Topic\n\n### 1.1 Q?\n\nid: 11111111-1111-1111-1111-111111111111\n\ndepends: widget\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n";
        let with_id = rf_from(base);
        let no_id = rf_from(&base.replace("id: 11111111-1111-1111-1111-111111111111\n\n", ""));
        assert_ne!(plan_hash(&with_id), plan_hash(&no_id));
        let swapped = base.replace("11111111-1111-1111-1111-111111111111", "22222222-2222-2222-2222-222222222222");
        assert_ne!(plan_hash(&with_id), plan_hash(&rf_from(&swapped)), "swapping an id must move the hash");
    }

    #[test]
    fn derive_tiers_resolves_a_heading_dep_written_as_an_id() {
        // Post-approval form: the depends line already names the target's
        // id rather than its heading text; tiers must still derive from it,
        // and a self-dep by id must still be caught.
        let rf = rf_from(
            "# Topic\n\n\
             ### A?\n\nid: 11111111-1111-1111-1111-111111111111\n\ndepends: widget\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
             ### B?\n\nid: 22222222-2222-2222-2222-222222222222\n\ndepends: 11111111-1111-1111-1111-111111111111\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        );
        assert_eq!(rf.tiers, Some(vec![1, 2]), "{:?}", rf.derive_error);
        assert!(rf.derive_error.is_none());

        let selfdep = rf_from(
            "# Topic\n\n### A?\n\nid: 11111111-1111-1111-1111-111111111111\n\ndepends: 11111111-1111-1111-1111-111111111111\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        );
        match &selfdep.derive_error {
            Some(DeriveError::SelfDep(h)) => assert_eq!(h, "A?"),
            other => panic!("expected SelfDep, got {:?}", other),
        }
    }

    #[test]
    fn derive_tiers_treats_an_unmatched_id_as_a_non_heading_reference() {
        // An `Id` entry that names no heading in this file (a term or a
        // decision elsewhere) must never be a Dangling error: only `Text`
        // heading-shaped entries can dangle.
        let rf = rf_from(
            "# Topic\n\n### A?\n\ndepends: 33333333-3333-3333-3333-333333333333\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        );
        assert!(rf.derive_error.is_none(), "{:?}", rf.derive_error);
        assert_eq!(rf.tiers, Some(vec![1]));
    }
}
