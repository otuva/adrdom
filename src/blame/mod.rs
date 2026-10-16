//! `adrdom/blame/<name>.md`: for one origin, its line ranges and whether each
//! range holds a decision or only detail (per "What does a blame file hold
//! of its origin?", d2851ea1 A; "blame", 108053a8-c2c3-4561-874c-2f573a5409f2).
//! Parsing, coverage, what one file waits for and the `Index` of every blame
//! file live here; `actions::do_approve` dispatches a path under
//! `repo::BLAME_DIR` to `actions::do_approve_blame`, the same way it
//! dispatches `repo::TERMS_PATH` to `do_approve_terms`.
//!
//! `check` holds a blame file to its origin's blob (6f6e7f4a A) and to its
//! origin staying until the file is approved (0c3d498a A) --
//! `commands::check::blame`; `show` prints a blame file in the form every
//! file approve takes is printed (91f9652a A, 1af76f91 A -- `commands::show`).
mod coverage;
mod files;
mod origin;
mod rows;

pub use coverage::{coverage_errors, BeyondError, GapError};
pub use files::{load_all_with_errors, Entry, Index};
pub use origin::{plain_path, valid_origin_path};
pub use rows::{decision_rows, file_state, FileState};

use crate::msg;
use crate::record;
use std::path::PathBuf;

/// What a row says of its range when the range is a decision the owner
/// settles by picking an option (d2851ea1 A).
const DECISION: &str = "decision";
/// What a row says of a range that only the code or the origin records.
const DETAIL: &str = "detail";
/// Opens the id line of a blame file, on line 3.
const ID_PREFIX: &str = "id: ";

/// One row of a blame file, as written: the origin's 1-based inclusive line
/// range and what it holds, `decision` or `detail` (d2851ea1 A), trimmed. A
/// row with any other value is a shape fault of the file and stays here, so
/// coverage still reads its range.
#[derive(Debug, Clone)]
pub struct Row {
    pub start: usize,
    pub end: usize,
    pub value: String,
}

impl Row {
    pub fn is_decision(&self) -> bool {
        self.value == DECISION
    }
}

/// Which rule a fault of a blame file's shape breaks, carried beside its
/// text so that no rule is ever read off the wording.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShapeKind {
    /// The structure of the file: a header line missing, the blank line
    /// after the header, a line that is no row.
    File,
    /// A row whose value is neither `decision` nor `detail`.
    RowValue,
    /// An id line that holds no identifier, or sits below the header: the
    /// tool writes the identifier, at approval (d2851ea1 A).
    Id,
}

/// One fault of a blame file's shape: `text` is what is found, naming its
/// line, and `kind` the rule it breaks.
#[derive(Debug, Clone)]
pub struct ShapeError {
    pub kind: ShapeKind,
    pub text: String,
}

impl ShapeError {
    /// The message of this fault on the blame file `rel`.
    pub fn said(&self, rel: &str, approved: Option<&str>) -> msg::Said {
        match self.kind {
            ShapeKind::File => msg::blame_shape_error(rel, &self.text),
            ShapeKind::RowValue => msg::blame_row_value_error(rel, &self.text),
            ShapeKind::Id => msg::blame_id_error(rel, &self.text, approved),
        }
    }
}

#[derive(Debug, Clone)]
pub struct BlameFile {
    pub path: PathBuf,
    pub rel: String,
    pub lines: Vec<String>,
    pub origin: Option<String>,
    pub blob: Option<String>,
    /// The identifier on line 3, when the file has one and it is one.
    pub id: Option<String>,
    pub rows: Vec<Row>,
    pub shape_errors: Vec<ShapeError>,
}

/// A git object id: 40 hex characters (sha1) or 64 (sha256, per a
/// `--object-format=sha256` repository).
pub(crate) fn is_hex_oid(s: &str) -> bool {
    (s.len() == 40 || s.len() == 64) && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// `"<start>-<end>\t<value>"`: `None` when the line doesn't have that shape
/// at all (no tab, a range that is not digits, a dash and digits as a blame
/// line writes it -- `record::range_token` -- empty value, or `end < start`).
/// The value is trimmed and not judged here.
fn parse_row(line: &str) -> Option<(usize, usize, String)> {
    let (range, value) = line.split_once('\t')?;
    let value = value.trim();
    if value.is_empty() {
        return None;
    }
    let (start, end) = record::range_token(range.trim())?;
    Some((start, end, value.to_string()))
}

/// Parses a blame file's fixed shape (per "What does a blame file hold of
/// its origin?"): `origin: <path>` on line 1, `blob: <40 or 64 hex>` on line
/// 2, then a blank line, or an `id: <identifier>` line and then the blank
/// line, then one row per non-blank line after that. Anything else is a
/// shape error, collected here rather than returned at the first, the same
/// way `terms::parse`/`record::build` collect every shape error at once.
pub fn parse(path: PathBuf, rel: String, lines: Vec<String>) -> BlameFile {
    let mut errors = Vec::new();
    let mut fault = |kind: ShapeKind, text: String| errors.push(ShapeError { kind, text });

    let origin = match lines.first().and_then(|l| l.strip_prefix("origin: ")) {
        Some(v) if !v.trim().is_empty() => Some(v.trim().to_string()),
        _ => {
            fault(
                ShapeKind::File,
                "line 1: missing \"origin: <path>\"".to_string(),
            );
            None
        }
    };

    let blob = match lines.get(1).and_then(|l| l.strip_prefix("blob: ")) {
        Some(v) if is_hex_oid(v.trim()) => Some(v.trim().to_string()),
        _ => {
            fault(
                ShapeKind::File,
                "line 2: missing \"blob: <40 or 64 hex>\"".to_string(),
            );
            None
        }
    };

    // `approve` writes the id line (d2851ea1 A) into the blank line's place:
    // the blank line then follows it.
    let mut id = None;
    let mut blank = 2;
    if let Some(v) = lines.get(2).and_then(|l| l.strip_prefix(ID_PREFIX)) {
        let v = v.trim();
        if record::is_uuid(v) {
            id = Some(v.to_string());
        } else {
            fault(
                ShapeKind::Id,
                format!(
                    "line 3: id line holds \"{}\", which is no identifier the tool assigns",
                    v
                ),
            );
        }
        blank = 3;
    }
    if !lines.get(blank).is_some_and(|l| l.trim().is_empty()) {
        fault(
            ShapeKind::File,
            format!("line {}: expected a blank line after the header", blank + 1),
        );
    }

    let mut rows = Vec::new();
    for (i, l) in lines.iter().enumerate().skip(blank + 1) {
        if l.trim().is_empty() {
            continue;
        }
        if l.starts_with(ID_PREFIX) {
            fault(
                ShapeKind::Id,
                format!("line {}: id line below the header", i + 1),
            );
            continue;
        }
        match parse_row(l) {
            Some((start, end, value)) => {
                if value != DECISION && value != DETAIL {
                    fault(
                        ShapeKind::RowValue,
                        format!(
                            "line {}: row value \"{}\" is neither decision nor detail",
                            i + 1,
                            value
                        ),
                    );
                }
                rows.push(Row { start, end, value });
            }
            None => fault(
                ShapeKind::File,
                format!(
                    "line {}: not a row (\"<start>-<end>\", a tab, then decision or detail)",
                    i + 1
                ),
            ),
        }
    }

    BlameFile {
        path,
        rel,
        lines,
        origin,
        blob,
        id,
        rows,
        shape_errors: errors,
    }
}

/// The hash `state::blame_approved`/`do_approve_blame` compare against the
/// ledger's latest `blame` row for a path: the same normalization
/// `terms::file_hash` applies (trailing whitespace on a line never moves
/// the hash), over `lines` rather than a whole file's text so a caller that
/// already has the split lines (the copy with the id line in it, or a raw
/// file's content split the same way) never has to re-join and re-split.
pub fn file_hash(lines: &[String]) -> String {
    let text = lines
        .iter()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n");
    record::sha256_hex(&record::normalize(&text))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ledger::{Ledger, Row as LedgerRow};

    pub(super) const ZERO_BLOB: &str = "0000000000000000000000000000000000000000";
    pub(super) const ID_A: &str = "11111111-1111-4111-8111-111111111111";

    pub(super) fn lines_of(s: &str) -> Vec<String> {
        record::split_lines(s)
    }

    pub(super) fn sample() -> String {
        format!(
            "origin: docs/x.md\nblob: {}\n\n1-3\tdecision\n4-4\tdetail\n",
            ZERO_BLOB
        )
    }

    pub(super) fn with_id(id: &str) -> String {
        sample().replacen("\n\n", &format!("\nid: {}\n\n", id), 1)
    }

    pub(super) fn parsed(rel: &str, s: &str) -> BlameFile {
        parse(PathBuf::from(rel), rel.to_string(), lines_of(s))
    }

    pub(super) fn texts(bf: &BlameFile) -> Vec<&str> {
        bf.shape_errors.iter().map(|e| e.text.as_str()).collect()
    }

    pub(super) fn ledger_approving(files: &[&BlameFile]) -> Ledger {
        Ledger {
            rows: files
                .iter()
                .map(|bf| LedgerRow {
                    time: "2024-01-01T00:00:00Z".to_string(),
                    event: "blame".to_string(),
                    id: bf.id.clone().unwrap_or_else(|| "-".to_string()),
                    hash: file_hash(&bf.lines),
                    detail: bf.rel.clone(),
                    version: "0.1.0".to_string(),
                })
                .collect(),
            malformed: vec![],
        }
    }
    // adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
    #[test]
    fn well_formed_file_parses_with_no_shape_errors() {
        let bf = parse(
            PathBuf::from("x.md"),
            "adrdom/blame/x.md".to_string(),
            lines_of(&sample()),
        );
        assert!(bf.shape_errors.is_empty(), "{:?}", bf.shape_errors);
        assert_eq!(bf.origin.as_deref(), Some("docs/x.md"));
        assert_eq!(bf.blob.as_deref(), Some(ZERO_BLOB));
        assert_eq!(bf.id, None);
        assert_eq!(bf.rows.len(), 2);
        assert_eq!((bf.rows[0].start, bf.rows[0].end), (1, 3));
        assert!(bf.rows[0].is_decision());
        assert!(!bf.rows[1].is_decision());
        assert_eq!(bf.rows[1].value, DETAIL);
    }

    // adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
    #[test]
    fn a_row_value_is_trimmed() {
        let s = sample().replacen("1-3\tdecision", "1-3\t  decision  ", 1);
        let bf = parsed("x.md", &s);
        assert!(bf.shape_errors.is_empty(), "{:?}", bf.shape_errors);
        assert_eq!(bf.rows[0].value, "decision");
    }

    // adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
    #[test]
    fn missing_origin_is_a_shape_error() {
        let s = sample().replacen("origin: docs/x.md\n", "not an origin line\n", 1);
        let bf = parsed("x.md", &s);
        assert!(bf
            .shape_errors
            .iter()
            .any(|e| e.text.contains("origin") && e.kind == ShapeKind::File));
    }

    // adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
    #[test]
    fn a_short_blob_is_a_shape_error() {
        let s = sample().replacen(&format!("blob: {}\n", ZERO_BLOB), "blob: abc\n", 1);
        let bf = parsed("x.md", &s);
        assert!(bf
            .shape_errors
            .iter()
            .any(|e| e.text.contains("blob") && e.kind == ShapeKind::File));
    }

    // A sha256-object-format repo's blob ids are 64 hex characters, not 40.
    #[test]
    fn a_64_hex_blob_is_accepted() {
        let s = sample().replacen(
            &format!("blob: {}\n", ZERO_BLOB),
            &format!("blob: {}\n", "0".repeat(64)),
            1,
        );
        let bf = parsed("x.md", &s);
        assert!(bf.shape_errors.is_empty(), "{:?}", bf.shape_errors);
        assert_eq!(bf.blob.as_deref(), Some("0".repeat(64).as_str()));
    }

    // adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
    #[test]
    fn a_row_with_no_tab_is_a_shape_error() {
        let s = sample().replacen("1-3\tdecision\n", "1-3 decision\n", 1);
        let bf = parsed("x.md", &s);
        assert_eq!(texts(&bf).len(), 1);
        assert!(
            texts(&bf)[0].contains("line 4: not a row"),
            "{:?}",
            texts(&bf)
        );
        assert_eq!(bf.shape_errors[0].kind, ShapeKind::File);
    }

    // adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
    #[test]
    fn a_missing_blank_line_after_the_header_is_a_shape_error() {
        let s = sample().replacen("\n\n1-3", "\n1-3", 1);
        let bf = parsed("x.md", &s);
        assert!(
            texts(&bf).contains(&"line 3: expected a blank line after the header"),
            "{:?}",
            texts(&bf)
        );
    }

    // adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
    // A row whose value is neither word keeps its range, so coverage still
    // reads it, and its fault carries the kind of its own rule.
    #[test]
    fn a_row_value_that_is_neither_decision_nor_detail_keeps_its_range() {
        for old in [
            "term: pass",
            "dropped: dead",
            "prose: example",
            "free text",
            "Decision",
        ] {
            let s = sample().replacen("1-3\tdecision", &format!("1-3\t{}", old), 1);
            let bf = parsed("x.md", &s);
            assert_eq!(bf.rows.len(), 2, "{}", old);
            assert_eq!((bf.rows[0].start, bf.rows[0].end), (1, 3));
            assert!(!bf.rows[0].is_decision());
            assert_eq!(bf.shape_errors.len(), 1, "{}: {:?}", old, bf.shape_errors);
            assert_eq!(bf.shape_errors[0].kind, ShapeKind::RowValue);
            assert_eq!(
                bf.shape_errors[0].text,
                format!(
                    "line 4: row value \"{}\" is neither decision nor detail",
                    old
                )
            );
        }
    }

    // adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
    #[test]
    fn an_id_line_on_line_3_is_read_and_the_blank_line_follows_it() {
        let bf = parsed("x.md", &with_id(ID_A));
        assert!(bf.shape_errors.is_empty(), "{:?}", bf.shape_errors);
        assert_eq!(bf.id.as_deref(), Some(ID_A));
        assert_eq!(bf.rows.len(), 2);
    }

    // adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
    #[test]
    fn an_id_line_with_no_blank_line_after_it_is_a_shape_error() {
        let s = with_id(ID_A).replacen("\n\n1-3", "\n1-3", 1);
        let bf = parsed("x.md", &s);
        assert_eq!(
            texts(&bf),
            vec!["line 4: expected a blank line after the header"]
        );
        assert_eq!(bf.shape_errors[0].kind, ShapeKind::File);
    }

    // adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
    #[test]
    fn an_id_line_that_is_no_identifier_is_refused_with_the_id_kind() {
        let bf = parsed("x.md", &with_id("not-an-id"));
        assert_eq!(
            texts(&bf),
            vec!["line 3: id line holds \"not-an-id\", which is no identifier the tool assigns"]
        );
        assert_eq!(bf.shape_errors[0].kind, ShapeKind::Id);
        assert_eq!(bf.id, None);
        assert_eq!(bf.rows.len(), 2, "the rows after it are still read");
    }

    // adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
    #[test]
    fn an_id_line_below_the_header_is_refused_with_the_id_kind() {
        let s = sample() + &format!("id: {}\n", ID_A);
        let bf = parsed("x.md", &s);
        assert_eq!(texts(&bf), vec!["line 6: id line below the header"]);
        assert_eq!(bf.shape_errors[0].kind, ShapeKind::Id);
        assert_eq!(bf.id, None);
        assert_eq!(bf.rows.len(), 2);
    }

    #[test]
    fn parse_row_rejects_an_inverted_range() {
        assert!(parse_row("5-3\tdetail").is_none());
    }

    // adr: d2851ea1-fcb6-476a-b321-c813a594d5e5
    // A range is digits, a dash and digits, as on a blame line: a sign or a
    // space inside it makes the line no row.
    #[test]
    fn a_row_range_is_digits_a_dash_and_digits() {
        assert_eq!(parse_row("1-3\tdetail"), Some((1, 3, "detail".to_string())));
        for bad in [
            "+1-+3", "+1-3", "1-+3", "-1-3", "1 - 3", "0-2", "1-x", "1-2-3",
        ] {
            assert!(parse_row(&format!("{}\tdetail", bad)).is_none(), "{}", bad);
        }
        let bf = parsed(
            "x.md",
            &sample().replacen("1-3\tdecision", "+1-+3\tdecision", 1),
        );
        assert_eq!(texts(&bf).len(), 1, "{:?}", texts(&bf));
        assert!(
            texts(&bf)[0].contains("line 4: not a row"),
            "{:?}",
            texts(&bf)
        );
        assert_eq!(bf.rows.len(), 1);
    }
}
