use crate::terms;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

mod body;
mod depends;
mod hash;
mod parse;
mod tiers;

pub use hash::{
    assumes_hash, body_hash, body_text, canonical_set, context_hash, derive_file_hash, hash_range,
    heading_hash, hex, plan_hash, question_hash, question_hashes, section_hash, set_suffix,
    sha256_hex, version_of, VERSION_LEN,
};
use parse::analyze_shape;
pub(crate) use parse::{detail_line, parse_option_line, Detail};
pub use parse::{non_question_heading_lines, parse_sections};
pub use tiers::{derive_tiers, in_file_parents, DEPENDS_LINE_CAP, STAND_CAP};

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
/// words (per "What does a reference in a topic file carry?"): `Id` is a uuid already written to
/// the file, resolved by lookup (a heading id in this file, a term id, or a decision id) rather
/// than by shape, since `approve` is the only command that mints an id and rewrites a `depends:`
/// line to one. `Text` is anything still unresolved: heading-shaped (ends in `?`, optionally
/// carrying an answer set) or term-shaped otherwise, exactly as `approve` classifies it when
/// rewriting. Term/decision/heading resolution for an `Id` or a term-shaped `Text` happens in the
/// commands (`approve`/`check`), not here: record.rs has no ledger or terms access; a
/// heading-shaped reference (either form) is resolved locally, in memory, against this file's own
/// sections (used by `derive_tiers`/`dependents_of` so `show` can print tiers even before a plan is
/// approved).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Dep {
    Id {
        id: String,
        set: Option<AnswerSet>,
    },
    Text {
        text: String,
        set: Option<AnswerSet>,
    },
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

/// One entry of a `blame:` line (per "What does a question carry of the blame
/// file it came from?"): the name of a blame file, or its identifier once
/// approve has turned it, and the row ranges of that file the question was
/// born from, each `<from>-<to>` with `from >= 1` and `to >= from`. An entry
/// is split from the right: the trailing run of range tokens is its ranges,
/// what stands before them its name, and the first token is always the name,
/// so a missing range leaves a name alone. Nothing resolves here: record.rs has no
/// blame files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlameEntry {
    pub file: String,
    pub ranges: Vec<(usize, usize)>,
}

impl BlameEntry {
    /// The entry written as `text`, already split from its line at the commas.
    pub fn parse(text: &str) -> BlameEntry {
        let tokens: Vec<&str> = text.split_whitespace().collect();
        // The first token is always the name, even when it reads as a range
        // (a blame file may be called `10-14`).
        let name_len = tokens
            .iter()
            .rposition(|t| range_token(t).is_none())
            .map_or(0, |i| i + 1);
        let name_len = name_len.max(tokens.len().min(1));
        BlameEntry {
            file: tokens[..name_len].join(" "),
            ranges: tokens[name_len..]
                .iter()
                .filter_map(|t| range_token(t))
                .collect(),
        }
    }

    /// The entry as a line writes it, single spaces between its parts:
    /// what a finding quotes.
    pub fn written(&self) -> String {
        let ranges = self
            .ranges
            .iter()
            .map(|(from, to)| format!("{}-{}", from, to));
        std::iter::once(self.file.clone())
            .filter(|f| !f.is_empty())
            .chain(ranges)
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// `<from>-<to>` in digits alone, `from >= 1` and `to >= from`: the range of
/// a `blame:` entry and of a row of a blame file alike.
pub(crate) fn range_token(token: &str) -> Option<(usize, usize)> {
    let (from, to) = token.split_once('-')?;
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !digits(from) || !digits(to) {
        return None;
    }
    let (from, to) = (from.parse::<usize>().ok()?, to.parse::<usize>().ok()?);
    (from >= 1 && to >= from).then_some((from, to))
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
}

#[derive(Debug, Clone)]
pub struct Section {
    pub start: usize,
    pub end: usize,
    pub heading_text: String,
    pub id: Option<String>,
    /// `supersedes: ` ids, comma-separated in the source, in file order (4.14).
    pub supersedes: Vec<String>,
    /// `blame: ` entries, comma-separated in the source, in file order (per
    /// "What does a question carry of the blame file it came from?").
    pub blame: Vec<BlameEntry>,
    /// `depends: ` entries, in file order (8.3/8.4).
    pub depends: Vec<Dep>,
    pub options: Vec<OptionRec>,
    pub answer_line: Option<usize>,
    pub answer_value: Option<String>,
    /// Errors from the parser: duplicate id line, an option
    /// with no decision sentence, and an answer naming a missing option.
    /// The option-count rules ("at most three" / "at least two") live in
    /// `shape_errors` instead, since `approve` must refuse on
    /// them before ever touching the file, same as every other shape rule.
    pub parse_errors: Vec<String>,
    /// Errors from the topic-file-shape rules (4.9/4.7/3.4/8.3): stray
    /// lines in a body, a non-skeleton question missing its
    /// Problem/Standard lines, an option count outside two-to-three, an
    /// option missing its Why/Costs line, and a malformed `depends:` line.
    /// `approve` both refuse on this slice (plus the file-level
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
            Dep::Id { id, .. } => self
                .sections
                .iter()
                .position(|s| s.id.as_deref() == Some(id.as_str())),
            Dep::Text { text, .. } => {
                let t = text.trim();
                if !t.ends_with('?') {
                    return None;
                }
                self.sections
                    .iter()
                    .position(|s| s.heading_text.trim() == t)
            }
        }
    }

    /// Sections whose heading `depends:` entry names section `idx`'s
    /// heading (by id once approved, by heading text before), paired with
    /// the set they declare on that entry, if any (per "What does a
    /// depends line name?", updated by "What does a reference in a topic file carry?":
    /// dependents_of keys on heading ids once they exist).
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
pub fn load_all_via(
    root: &Path,
    source: &crate::repo::Source,
) -> (Vec<RecordFile>, Vec<(PathBuf, String)>) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::{build, split_lines, RecordFile};
    use std::path::PathBuf;

    pub(super) fn rf_from(content: &str) -> RecordFile {
        let lines = split_lines(content);
        build(PathBuf::from("test.md"), "test.md".to_string(), lines)
    }

    pub(super) const SHAPED: &str = "# Topic\n\ncontext prose line one.\ncontext prose line two.\n\n### 1.1 First question?\n\nProblem: p.\n\nStandard: s.\n\n- A) Do the thing.\n  Why: reasons.\n  Costs: costs.\n- B) Do the other thing.\n  Why: other reasons.\n  Costs: other costs.\n";

    fn entry(text: &str) -> (String, Vec<(usize, usize)>) {
        let e = BlameEntry::parse(text);
        (e.file, e.ranges)
    }

    // adr: 69708a07-469c-406b-9e25-63da1605b432
    // A blame entry is split from the right: the trailing run of range
    // tokens is its ranges, what stands before them its name.
    #[test]
    fn a_blame_entry_is_a_name_and_its_row_ranges() {
        assert_eq!(
            entry("old-file 10-14"),
            ("old-file".to_string(), vec![(10, 14)])
        );
        assert_eq!(
            entry("old-file 10-14 31-35 3-3"),
            ("old-file".to_string(), vec![(10, 14), (31, 35), (3, 3)])
        );
        assert_eq!(
            entry("sub/other   3-9"),
            ("sub/other".to_string(), vec![(3, 9)])
        );
        let id = "69708a07-469c-406b-9e25-63da1605b432";
        assert_eq!(
            entry(&format!("{} 1-2", id)),
            (id.to_string(), vec![(1, 2)])
        );
    }

    // adr: 69708a07-469c-406b-9e25-63da1605b432
    #[test]
    fn a_blame_entry_may_lack_its_ranges_or_its_name() {
        assert_eq!(entry("old-file"), ("old-file".to_string(), vec![]));
        // The first token is always the name, even when it reads as a range.
        assert_eq!(entry("10-14 31-35"), ("10-14".to_string(), vec![(31, 35)]));
        assert_eq!(entry("10-14"), ("10-14".to_string(), vec![]));
        // A token that is no range (`5-3`, `0-2`, `1-x`, `+1-2`) is part of the name.
        for bad in ["5-3", "0-2", "1-x", "+1-2", "1-2-3"] {
            let text = format!("old-file {}", bad);
            assert_eq!(entry(&text), (text.clone(), vec![]), "{}", bad);
        }
        assert_eq!(
            entry("old-file 5-3 1-2"),
            ("old-file 5-3".to_string(), vec![(1, 2)])
        );
    }

    // adr: 69708a07-469c-406b-9e25-63da1605b432
    #[test]
    fn a_blame_entry_is_quoted_as_the_line_writes_it() {
        for text in ["old-file 10-14 31-35", "old-file", "10-14"] {
            assert_eq!(BlameEntry::parse(text).written(), text);
        }
        assert_eq!(
            BlameEntry::parse("old-file   10-14").written(),
            "old-file 10-14"
        );
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
        assert_eq!(
            deps.len(),
            2,
            "{:?}",
            deps.iter().map(|(i, _)| i).collect::<Vec<_>>()
        );
        assert!(deps
            .iter()
            .any(|(i, set)| *i == 1 && *set == Some(&AnswerSet::Letters(vec!['A']))));
        assert!(deps.iter().any(|(i, set)| *i == 2 && set.is_none()));
    }
}
