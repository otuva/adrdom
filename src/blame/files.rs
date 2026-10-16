//! The index of every blame file in a source, and the entries it holds.

use super::{parse, BlameFile, Row};
use crate::ledger::Ledger;
use crate::record;
use crate::repo;
use crate::status::{self, FileStatus};
use std::path::Path;
/// One blame file of the `Index`: parsed, with its name and its status.
#[derive(Debug, Clone)]
pub struct Entry {
    /// Its path under `repo::BLAME_DIR` without `.md`: `pass/notes` for
    /// `adrdom/blame/pass/notes.md`.
    pub name: String,
    /// Against its latest approval (`status::blame_file_status`).
    pub status: FileStatus,
    pub file: BlameFile,
}

impl Entry {
    /// Its path from the repository root.
    pub fn rel(&self) -> &str {
        &self.file.rel
    }

    /// The identifier on its id line, when it has one.
    pub fn id(&self) -> Option<&str> {
        self.file.id.as_deref()
    }

    pub fn rows(&self) -> &[Row] {
        &self.file.rows
    }
}

/// Every blame file under `repo::BLAME_DIR`, nested ones too, sorted by
/// path: the one list that anything resolving a blame file by its name or
/// its identifier reads (per "What does a question carry of the blame file
/// it came from?", 672201dd A). `check` at a commit reads it through the
/// staged files, every other command through the working tree.
#[derive(Debug, Clone)]
pub struct Index {
    entries: Vec<Entry>,
    /// The path and identifier of every `blame` ledger row, in ledger order.
    blame_rows: Vec<(String, String)>,
}

impl Index {
    /// The index of `files`, already parsed, each with its status read from
    /// the ledger `led`.
    pub fn of(mut files: Vec<BlameFile>, led: &Ledger) -> Index {
        files.sort_by(|a, b| a.rel.cmp(&b.rel));
        let entries = files
            .into_iter()
            .map(|file| Entry {
                name: name_of(&file.rel),
                status: status::blame_file_status(led, &file.rel, &record::join_lines(&file.lines)),
                file,
            })
            .collect();
        let blame_rows = led
            .rows
            .iter()
            .filter(|r| r.event == "blame")
            .map(|r| (r.detail.clone(), r.id.clone()))
            .collect();
        Index {
            entries,
            blame_rows,
        }
    }

    /// The identifier of the latest `blame` row for `rel`; `None` when the
    /// file has no approval, or when its latest row is one of an older ledger
    /// (identifier `-`): approved, with no identifier assigned yet.
    pub fn approved_id(&self, rel: &str) -> Option<&str> {
        self.blame_rows
            .iter()
            .rfind(|(path, _)| path == rel)
            .map(|(_, id)| id.as_str())
            .filter(|id| *id != "-")
    }

    /// True when some `blame` row, for any path, holds `id`.
    pub fn row_holds(&self, id: &str) -> bool {
        self.blame_rows
            .iter()
            .any(|(_, held)| held.eq_ignore_ascii_case(id))
    }

    /// The index of every blame file in `source`, and the path of each one
    /// that could not be read as text (left out of the index).
    pub fn read(root: &Path, source: &repo::Source, led: &Ledger) -> (Index, Vec<String>) {
        let mut files = Vec::new();
        let mut unreadable = Vec::new();
        for rel in rels(root, source) {
            match source.read(root, &rel) {
                Some(content) => {
                    files.push(parse(root.join(&rel), rel, record::split_lines(&content)))
                }
                None => unreadable.push(rel),
            }
        }
        (Index::of(files, led), unreadable)
    }

    /// Every blame file, by path.
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// The blame file called `name` (`Entry::name`).
    pub fn by_name(&self, name: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.name == name)
    }

    /// The blame file a `blame:` entry names: the one that keeps the
    /// identifier `written` when it is one, else the one called `written`.
    pub fn named(&self, written: &str) -> Option<&Entry> {
        if record::is_uuid(written) {
            self.keeper_of(written)
        } else {
            self.by_name(written)
        }
    }

    /// The blame file that keeps `id` when several hold it: the first
    /// approved one, else the first whose path the ledger holds an approval
    /// for (changed), else the first by path. A file approved once never
    /// loses its identifier to a copy made of it, edited or not.
    pub fn keeper_of(&self, id: &str) -> Option<&Entry> {
        let holds = |e: &&Entry| e.id().is_some_and(|held| held.eq_ignore_ascii_case(id));
        [FileStatus::Approved, FileStatus::Changed]
            .into_iter()
            .find_map(|status| {
                self.entries
                    .iter()
                    .filter(holds)
                    .find(|e| e.status == status)
            })
            .or_else(|| self.entries.iter().find(holds))
    }

    /// The blame file whose identifier `bf` holds, when another file keeps
    /// it (`keeper_of`); `None` when `bf` has no id line or keeps its id.
    pub fn id_taken_from(&self, bf: &BlameFile) -> Option<&Entry> {
        let keeper = self.keeper_of(bf.id.as_deref()?)?;
        (keeper.rel() != bf.rel).then_some(keeper)
    }
}

/// The name of the blame file at `rel`: `Entry::name`.
fn name_of(rel: &str) -> String {
    let under = rel
        .strip_prefix(repo::BLAME_DIR)
        .map_or(rel, |r| r.trim_start_matches('/'));
    under.strip_suffix(".md").unwrap_or(under).to_string()
}

/// Every `*.md` file under `repo::BLAME_DIR` in `source`, nested ones too,
/// sorted: the listing `check` judges and approve takes.
fn rels(root: &Path, source: &repo::Source) -> Vec<String> {
    let mut rels: Vec<String> = source
        .list_under(root, repo::BLAME_DIR)
        .into_iter()
        .filter(|p| p.ends_with(".md"))
        .collect();
    rels.sort();
    rels
}

/// Every `*.md` file under `repo::BLAME_DIR`, parsed, off the working tree,
/// and every file that failed to read at all (`(rel, io error text)`), so a
/// caller (`show`) can print a line for it instead of silently omitting it
/// -- fail-open would otherwise hide a blame file's own read failure from
/// the one command that has no `check` category to catch it.
pub fn load_all_with_errors(root: &Path) -> (Vec<BlameFile>, Vec<(String, String)>) {
    let mut out = Vec::new();
    let mut errors = Vec::new();
    for rel in rels(root, &repo::Source::WorkingTree) {
        let p = root.join(&rel);
        match std::fs::read_to_string(&p) {
            Ok(content) => out.push(parse(p.clone(), rel, record::split_lines(&content))),
            Err(e) => errors.push((rel, e.to_string())),
        }
    }
    (out, errors)
}

#[cfg(test)]
mod tests {
    use super::super::tests::*;
    use super::*;

    // adr: 69708a07-469c-406b-9e25-63da1605b432
    // The index names each file by its path under adrdom/blame without
    // `.md`, nested files too, and lists them by path.
    #[test]
    fn the_index_names_each_file_and_lists_them_by_path() {
        let nested = parsed("adrdom/blame/pass/notes.md", &sample());
        let b = parsed("adrdom/blame/b.md", &sample());
        let a = parsed("adrdom/blame/a.md", &sample());
        let index = Index::of(vec![nested, b, a], &ledger_approving(&[]));
        let rels: Vec<&str> = index.entries().iter().map(|e| e.rel()).collect();
        assert_eq!(
            rels,
            vec![
                "adrdom/blame/a.md",
                "adrdom/blame/b.md",
                "adrdom/blame/pass/notes.md"
            ]
        );
        assert_eq!(
            index.by_name("pass/notes").map(|e| e.rel()),
            Some("adrdom/blame/pass/notes.md")
        );
        assert!(index.by_name("adrdom/blame/a").is_none());
        assert_eq!(index.by_name("a").map(|e| e.rows().len()), Some(2));
        assert!(index.entries().iter().all(|e| e.status == FileStatus::New));
    }

    // adr: 69708a07-469c-406b-9e25-63da1605b432
    // Text that is an identifier names a blame file by its id line alone, and
    // any other text by its name alone: a file whose name is another file's
    // identifier is not found by it.
    #[test]
    fn an_identifier_on_a_blame_line_resolves_as_an_identifier() {
        const ID_B: &str = "22222222-2222-4222-8222-222222222222";
        let named_like_an_id = parsed(&format!("adrdom/blame/{}.md", ID_A), &with_id(ID_B));
        let x = parsed("adrdom/blame/x.md", &with_id(ID_A));
        let index = Index::of(vec![named_like_an_id, x], &ledger_approving(&[]));
        let rel = |written: &str| index.named(written).map(|e| e.rel().to_string());
        assert_eq!(
            rel(ID_A).as_deref(),
            Some("adrdom/blame/x.md"),
            "the identifier, not the file called so"
        );
        assert_eq!(rel(ID_B), Some(format!("adrdom/blame/{}.md", ID_A)));
        assert_eq!(rel("x").as_deref(), Some("adrdom/blame/x.md"));
        assert_eq!(rel("22222222-2222-4222-8222-333333333333"), None);
    }

    // adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
    // One identifier names one file: the first approved holder keeps it,
    // else the first changed one (approved once), else the first by path,
    // and every other holder is the copy.
    #[test]
    fn the_first_approved_holder_keeps_an_identifier_else_the_first_by_path() {
        let a = parsed("adrdom/blame/a.md", &with_id(ID_A));
        let b = parsed("adrdom/blame/b.md", &with_id(ID_A));
        let c = parsed("adrdom/blame/c.md", &sample());

        let drafts = Index::of(
            vec![c.clone(), b.clone(), a.clone()],
            &ledger_approving(&[]),
        );
        assert_eq!(
            drafts.keeper_of(ID_A).map(|e| e.rel()),
            Some("adrdom/blame/a.md")
        );
        assert!(drafts.id_taken_from(&a).is_none());
        assert_eq!(
            drafts.id_taken_from(&b).map(|e| e.rel()),
            Some("adrdom/blame/a.md")
        );
        assert!(
            drafts.id_taken_from(&c).is_none(),
            "a file with no id line took none"
        );

        let original_is_b = Index::of(vec![c, b.clone(), a.clone()], &ledger_approving(&[&b]));
        assert_eq!(
            original_is_b.keeper_of(ID_A).map(|e| e.rel()),
            Some("adrdom/blame/b.md")
        );
        assert!(
            original_is_b.id_taken_from(&b).is_none(),
            "an approved original never gets a finding"
        );
        assert_eq!(
            original_is_b.id_taken_from(&a).map(|e| e.rel()),
            Some("adrdom/blame/b.md")
        );
        assert!(original_is_b
            .keeper_of("22222222-2222-4222-8222-222222222222")
            .is_none());

        // An approved original that was edited since is changed, and still
        // keeps the identifier over a copy never approved that sorts first.
        let edited_b = parsed(
            "adrdom/blame/b.md",
            &with_id(ID_A).replacen("4-4\tdetail", "4-4\tdecision", 1),
        );
        let edited = Index::of(vec![edited_b.clone(), a.clone()], &ledger_approving(&[&b]));
        assert_eq!(
            edited.by_name("b").map(|e| e.status),
            Some(FileStatus::Changed)
        );
        assert_eq!(
            edited.keeper_of(ID_A).map(|e| e.rel()),
            Some("adrdom/blame/b.md")
        );
        assert!(edited.id_taken_from(&edited_b).is_none());
        assert_eq!(
            edited.id_taken_from(&a).map(|e| e.rel()),
            Some("adrdom/blame/b.md")
        );
    }
}
