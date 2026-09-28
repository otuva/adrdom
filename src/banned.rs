use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

/// Where the hashed ban list lives, relative to the repo root: the one
/// constant for this path (defined in `repo.rs` alongside every other
/// governed path), used by every site that reads or writes it (`ban`,
/// `unban`, `ask`, `check --banned`, `terms_refusals`, `install`).
pub use crate::repo::BAN_LIST;

/// Same tokenization for `ban` and `check --banned`: the whole trimmed,
/// lowercased line, plus each maximal run of Unicode alphanumerics in it
/// (`crate::text::word_runs`).
pub fn tokenize(line: &str) -> Vec<String> {
    let trimmed = line.trim().to_lowercase();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let mut tokens = vec![trimmed.clone()];
    tokens.extend(crate::text::word_runs(&trimmed));
    tokens
}

/// Parses one line of `adrdom/state/banned.sha256`: the hash, and the term
/// uuid carried alongside it when the word was banned with `--term`.
fn parse_line(line: &str) -> (String, Option<String>) {
    let line = line.trim();
    match line.split_once(' ') {
        Some((hash, uuid)) => (hash.to_string(), Some(uuid.trim().to_string())),
        None => (line.to_string(), None),
    }
}

/// Formats one line of `adrdom/state/banned.sha256`: `<hash>` alone, or
/// `<hash> <uuid>` when a term uuid is carried.
pub fn format_line(hash: &str, uuid: &Option<String>) -> String {
    match uuid {
        Some(u) => format!("{} {}", hash, u),
        None => hash.to_string(),
    }
}

/// Every hash in the ban list, dropping the term uuid column.
pub fn read_set(path: &Path) -> HashSet<String> {
    read_entries(path).into_iter().map(|(h, _)| h).collect()
}

/// Every line of the ban list, in file order: the hash, and the term uuid
/// on the line, if any.
pub fn read_entries(path: &Path) -> Vec<(String, Option<String>)> {
    parse_entries(&fs::read_to_string(path).unwrap_or_default())
}

/// Parses ban-list entries straight from `content`, without touching the
/// filesystem: what `read_entries` does after its own
/// `fs::read_to_string`, split out so a caller reading through
/// `repo::Source` (`check`, so a staged edit to the ban list is judged
/// too) doesn't have to write the bytes to a temp file first.
pub fn parse_entries(content: &str) -> Vec<(String, Option<String>)> {
    content
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(parse_line)
        .collect()
}

/// Like `read_entries`, but through `source` (per "Does check at a commit
/// read the staged files?"): a staged edit to the ban list is judged, not
/// whatever is left on disk. Every caller that scans on `check`'s behalf
/// (directly or through `terms_refusals`) reads the list this way; every
/// other caller (`ban`, `unban`, `ask`, `approve`) keeps calling
/// `read_entries`, which always wants the working tree.
pub fn read_entries_via(root: &Path, source: &crate::repo::Source) -> Vec<(String, Option<String>)> {
    parse_entries(&source.read(root, BAN_LIST).unwrap_or_default())
}

/// One line's hit against the ban list: the 1-based line number within the
/// scanned text, the exact banned token as `tokenize` returns it, and the
/// term name when the hash's ban-list entry carries a uuid that still names
/// a term in `terms`.
pub struct Hit {
    pub line: usize,
    pub token: String,
    pub term: Option<String>,
}

/// The name of the term a banned-list entry was banned under: `uuid` is
/// the list's value for the hash (None for a word banned on its own), and
/// `terms` the terms file it may name. Shared by `hits` and `first_hit` so
/// every hit names its term the same way.
fn term_name(uuid: Option<&str>, terms: &[crate::terms::Term]) -> Option<String> {
    let uuid = uuid?;
    terms
        .iter()
        .find(|t| t.id.as_deref() == Some(uuid))
        .map(|t| t.name.clone())
}

/// The one banned-word scan: split `text` into lines, tokenize each line
/// with `tokenize`, hash every token and look it up in `hash_map` (a ban
/// list hash to the term uuid it was banned under, if any, exactly as
/// `read_entries`/`read_entries_via` return it, collected into a map). A
/// token repeated on the same line is reported once; every distinct hit is
/// returned in the order it appears. `terms` names a hit's term by its
/// uuid; pass `&[]` when no terms file applies (a hit then never carries a
/// name).
pub fn hits(
    text: &str,
    hash_map: &HashMap<String, Option<String>>,
    terms: &[crate::terms::Term],
) -> Vec<Hit> {
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let mut seen: HashSet<String> = HashSet::new();
        for tok in tokenize(line) {
            let h = crate::record::sha256_hex(&tok);
            let Some(uuid_opt) = hash_map.get(&h) else {
                continue;
            };
            if !seen.insert(tok.clone()) {
                continue;
            }
            let term = term_name(uuid_opt.as_deref(), terms);
            out.push(Hit { line: i + 1, token: tok, term });
        }
    }
    out
}

/// Like `hits`, but stops at the first hit instead of scanning the rest of
/// `text`: for a caller (`ask`'s candidate-body scan) that only acts on
/// whether a banned word is present at all, never on every place it
/// appears, so it never pays to hash every remaining token once one hit is
/// already enough to refuse.
pub fn first_hit(
    text: &str,
    hash_map: &HashMap<String, Option<String>>,
    terms: &[crate::terms::Term],
) -> Option<Hit> {
    for (i, line) in text.lines().enumerate() {
        for tok in tokenize(line) {
            let h = crate::record::sha256_hex(&tok);
            let Some(uuid_opt) = hash_map.get(&h) else {
                continue;
            };
            let term = term_name(uuid_opt.as_deref(), terms);
            return Some(Hit { line: i + 1, token: tok, term });
        }
    }
    None
}

/// Every hash the ledger currently says is banned (per "Does the tool
/// rewrite the ban list from the ledger?" and "Does check hold the ban list
/// equal to the ledger?"): for each hash that ever carries a `banned` or
/// `unbanned` row, its latest such row -- by the ledger's own order
/// (`ledger::parse` already sorts `rows` by time, never by file position) --
/// kept only when that latest row is `banned`, paired with the term uuid
/// from that row's `detail` column when it carries one. Shared by
/// `check_banned_ledger` (which only needs the hash) and `adr ban
/// --rebuild` (which needs the uuid too, to carry it onto the rewritten
/// file), so the two can never disagree about what "banned now" means.
pub fn currently_banned(led: &crate::ledger::Ledger) -> Vec<(String, Option<String>)> {
    let mut latest: HashMap<String, &crate::ledger::Row> = HashMap::new();
    for row in &led.rows {
        if row.event == "banned" || row.event == "unbanned" {
            latest.insert(row.hash.clone(), row);
        }
    }
    latest
        .into_values()
        .filter(|row| row.event == "banned")
        .map(|row| {
            let uuid = if row.detail.is_empty() { None } else { Some(row.detail.clone()) };
            (row.hash.clone(), uuid)
        })
        .collect()
}

/// The exact bytes `write_sorted` writes for `entries` -- sorted by hash
/// and, within a hash, by uuid (an entry with no uuid sorts first), then
/// deduplicated to one line per hash -- without touching the filesystem.
/// Split out from `write_sorted` so a caller that only wants to write when
/// the result would actually differ from what's already on disk (`ban`,
/// when it finds no new word to ban itself but the file may still be stale
/// against the ledger) can compare first instead of writing speculatively.
///
/// When two entries share one hash under different term uuids -- two
/// branches banning the same word under two different `--term` calls, then
/// merging -- which one survives can't depend on the order `entries` arrives
/// in, or the file a merge produces would depend on which side's row a
/// text merge happened to keep first. So entries are sorted by hash and
/// then, within a hash, by uuid, before `dedup_by` keeps the first of each
/// hash: the same input, in any order, always keeps the same uuid.
pub fn sorted_content(entries: &[(String, Option<String>)]) -> String {
    let mut entries = entries.to_vec();
    entries.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    entries.dedup_by(|a, b| a.0 == b.0);
    let mut content = String::new();
    for (hash, uuid) in &entries {
        content.push_str(&format_line(hash, uuid));
        content.push('\n');
    }
    content
}

/// Rewrites the whole ban list at `path` to `sorted_content(&entries)` (per
/// "Does the tool write the ban list in one fixed order?"): one line per
/// entry, no duplicate hashes. `ban` calls this with the ledger's own
/// banned set plus the newly banned ones folded in; `unban` calls it with
/// the existing entries minus the ones it removed; `--rebuild` calls it
/// with the ledger's banned set alone. All end up writing the same sorted,
/// deduplicated shape, so this is the one place that shape is written.
pub fn write_sorted(path: &Path, entries: Vec<(String, Option<String>)>) -> std::io::Result<()> {
    fs::write(path, sorted_content(&entries))
}
