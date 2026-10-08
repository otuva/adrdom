use crate::rules::Rule;
use super::ledger::{check_held, READER_MSGS};
use super::Failure;
use crate::msg;
use crate::reader;
use crate::repo::{self, Source};
use std::path::Path;

/// "Does check hold the reader's file as it holds the ledger?"
/// (26c5352f-3f82-4376-9dc2-8ccc0074d080 A): a malformed row fails, and the
/// rows at HEAD, at the other side of a merge and at `--since` stay byte for
/// byte, by the same rule as the ledger's. A missing file is fine: a fresh
/// repository has no rows.
pub(super) fn check_reader(root: &Path, source: &Source, since: Option<&str>) -> Vec<Failure> {
    let content = source.read(root, repo::READER_PATH).unwrap_or_default();
    let mut out: Vec<Failure> = reader::parse(&content)
        .malformed
        .iter()
        .map(|(lineno, raw)| Failure::new(Rule::Reader, msg::malformed_reader_row(*lineno, raw)))
        .collect();
    out.extend(check_held(root, repo::READER_PATH, &content, since, Rule::Reader, &READER_MSGS));
    out
}
