mod cycles;
mod lines;
mod parse;
mod resolve;

pub use cycles::cycles;
pub use lines::{
    format_terms_line, parse_terms_line, validate_pairs, write_terms_lines, Malformed, Problem,
    ProblemKind,
};
pub use parse::{parse, sentence_count};
pub use resolve::{definition_pairs, is_retired_phrase, phrase_lookup, resolve};

use crate::record::{normalize, sha256_hex};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct Term {
    pub name: String,
    pub id: Option<String>,
    pub definition: String,
    pub aliases: Vec<String>,
    pub retired: bool,
    /// 1-based line number of the `## name` heading.
    pub start_line: usize,
    /// 1-based line number of the `id: ` line, if present.
    pub id_line: Option<usize>,
    /// The machine-written pairing line (`terms: word=uuid, ...`), if
    /// present, exactly as it appears in the file.
    ///
    /// Read by `write_terms_lines` and by approve's dependents check.
    pub terms_line: Option<String>,
    /// 1-based line number of the `terms:` line, if present.
    pub terms_line_idx: Option<usize>,
}

#[derive(Clone)]
pub struct TermsFile {
    pub path: PathBuf,
    pub lines: Vec<String>,
    pub terms: Vec<Term>,
    pub errors: Vec<String>,
}

impl TermsFile {
    /// Every entry with no `id` field at all (per `parse`'s "missing id"
    /// error): the typed form of that same fact, so a caller that only
    /// needs to know whether one exists doesn't have to match on the error
    /// text.
    pub fn missing_ids(&self) -> Vec<&Term> {
        self.terms.iter().filter(|t| t.id.is_none()).collect()
    }
}

pub fn load(path: &Path) -> io::Result<TermsFile> {
    let content = fs::read_to_string(path)?;
    let lines = crate::record::split_lines(&content);
    let mut tf = parse(&lines);
    tf.path = path.to_path_buf();
    Ok(tf)
}

pub fn save(tf: &TermsFile) -> io::Result<()> {
    fs::write(&tf.path, tf.lines.join("\n"))
}

/// The text of the terms brief for `tf` (per "What does the terms brief
/// hold?", f1294391 A): one line per term that is not retired, in the
/// order of the terms file, `- name (alias, alias): definition`, the
/// brackets left out for a term with no aliases. The one place the text is
/// built: `approve` writes it and `check` compares against it.
pub fn brief_text(tf: &TermsFile) -> String {
    let mut out = String::new();
    for t in tf.terms.iter().filter(|t| !t.retired) {
        out.push_str("- ");
        out.push_str(&t.name);
        if !t.aliases.is_empty() {
            out.push_str(" (");
            out.push_str(&t.aliases.join(", "));
            out.push(')');
        }
        out.push_str(": ");
        out.push_str(&t.definition);
        out.push('\n');
    }
    out
}

/// The hash a `term` ledger row holds: sha256 over the entry's own text
/// (name, definition, aliases, retired flag), so an edit to any of them
/// changes it and nothing else (the id and the machine-written `terms:`
/// line are not part of it).
pub fn term_hash(t: &Term) -> String {
    let text = format!(
        "## {}\ndefinition: {}\naliases: {}\nretired: {}",
        t.name.trim_end(),
        t.definition.trim_end(),
        t.aliases.join(", ").trim_end(),
        if t.retired { "yes" } else { "no" }
    );
    sha256_hex(&normalize(&text))
}

pub fn file_hash(tf: &TermsFile) -> String {
    let text = tf
        .lines
        .iter()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n");
    sha256_hex(&normalize(&text))
}
/// Inserts `id: <uuid v4>` as the first line under each `## name` heading
/// that has none. Returns the number of ids inserted.
pub fn mint_ids(tf: &mut TermsFile) -> usize {
    let mut targets: Vec<usize> = tf
        .terms
        .iter()
        .filter(|t| t.id.is_none())
        .map(|t| t.start_line) // 1-based heading line; also the 0-based insertion index.
        .collect();
    targets.sort_unstable();
    targets.reverse(); // Bottom-to-top so earlier insertions don't shift later indices.
    let count = targets.len();
    for start_line in targets {
        let new_line = format!("id: {}", Uuid::new_v4());
        tf.lines.insert(start_line, new_line);
    }
    let reparsed = parse(&tf.lines);
    tf.terms = reparsed.terms;
    tf.errors = reparsed.errors;
    count
}

/// A clone of `tf` with an id minted in memory (`mint_ids`) for every entry
/// that lacks one; every id already present is kept as-is. Never written to
/// disk. `do_approve_terms` mints straight into the file it is about to
/// write; `check` (which never writes) calls this first so the same
/// id-dependent machinery -- `cycles`, `definition_pairs`, `phrase_lookup`
/// -- sees a terms file exactly as `approve` would build it, even one that
/// has never been approved and so carries no ids on disk yet. Without this,
/// `check` misses a cycle (and a moved/retired dependent pairing) that
/// `approve` would have refused on, since each of those only follows edges
/// between terms that have an id.
pub fn minted_copy(tf: &TermsFile) -> TermsFile {
    let mut copy = tf.clone();
    mint_ids(&mut copy);
    copy
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    pub(super) fn lines_of(s: &str) -> Vec<String> {
        crate::record::split_lines(s)
    }

    pub(super) fn well_formed() -> String {
        "# Terms\n\
\n\
## decision\n\
id: 11111111-1111-1111-1111-111111111111\n\
definition: A choice answered by the owner.\n\
aliases: choice\n\
\n\
## question\n\
id: 22222222-2222-2222-2222-222222222222\n\
definition: A thing the owner is asked to decide.\n\
aliases: \n"
            .to_string()
    }

    #[test]
    fn mint_ids_inserts_one_line_per_entry_then_nothing() {
        let s = well_formed()
            .replacen("id: 11111111-1111-1111-1111-111111111111\n", "", 1)
            .replacen("id: 22222222-2222-2222-2222-222222222222\n", "", 1);
        let mut tf = parse(&lines_of(&s));
        assert_eq!(tf.terms.len(), 2);
        assert!(tf.terms.iter().all(|t| t.id.is_none()));

        let inserted = mint_ids(&mut tf);
        assert_eq!(inserted, 2);
        assert!(tf.terms.iter().all(|t| t.id.is_some()));
        assert!(tf.errors.iter().all(|e| !e.contains("missing id")));

        let second = mint_ids(&mut tf);
        assert_eq!(second, 0);
    }

    #[test]
    fn load_save_and_file_hash_round_trip() {
        let dir = std::env::temp_dir().join(format!("adrdom-terms-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("terms.md");
        fs::write(&path, well_formed()).unwrap();

        let tf = load(&path).unwrap();
        assert!(tf.errors.is_empty(), "unexpected errors: {:?}", tf.errors);
        let h1 = file_hash(&tf);

        save(&tf).unwrap();
        let tf2 = load(&path).unwrap();
        let h2 = file_hash(&tf2);
        assert_eq!(h1, h2);

        fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn file_hash_ignores_trailing_whitespace_on_a_line() {
        let base = well_formed();
        let with_trailing_ws = base.replacen(
            "definition: A choice answered by the owner.\n",
            "definition: A choice answered by the owner.   \n",
            1,
        );
        let tf1 = parse(&lines_of(&base));
        let tf2 = parse(&lines_of(&with_trailing_ws));
        assert_eq!(file_hash(&tf1), file_hash(&tf2));
    }
}
