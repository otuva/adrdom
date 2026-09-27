//! One shared notion of "a word": a maximal run of alphanumeric
//! characters, no apostrophe or other exception. `banned::tokenize`,
//! `tells::words`, `terms::resolve`'s boundary check, and the page's
//! `mark_terms` all used to draw this boundary themselves, each slightly
//! differently (`serve`'s own version let an apostrophe join two words
//! into one, so "api's" paired "api" in `terms::resolve` but was not
//! marked as a term inside `mark_terms`); they all go through this module
//! now, so a word is the same thing everywhere.

/// Whether `c` is part of a word: plain alphanumeric, nothing else.
pub fn is_word_char(c: char) -> bool {
    c.is_alphanumeric()
}

/// Whether `a` and `b` hold the same lines once each line's trailing
/// whitespace is dropped: the one tolerance every governed-file-matches-its-
/// canonical-content check uses (the AGENTS.md brief section, and the git
/// hooks `adr install` writes and `adr check` verifies), so a file judged
/// current by one is never judged stale by the other over nothing but a
/// trailing space or a `\r` a checkout/editor left on a line.
pub fn lines_match_trimmed(a: &[&str], b: &[&str]) -> bool {
    a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| x.trim_end() == y.trim_end())
}

/// Every maximal run of word characters in `text`, in order, case
/// preserved.
pub fn word_runs(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for c in text.chars() {
        if is_word_char(c) {
            cur.push(c);
        } else if !cur.is_empty() {
            out.push(std::mem::take(&mut cur));
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_runs_splits_on_punctuation_and_whitespace() {
        assert_eq!(
            word_runs("Does check read staged files only?"),
            vec!["Does", "check", "read", "staged", "files", "only"]
        );
    }

    #[test]
    fn word_runs_treats_apostrophe_as_a_boundary() {
        assert_eq!(word_runs("api's"), vec!["api", "s"]);
    }

    #[test]
    fn word_runs_keeps_digits_in_a_word() {
        assert_eq!(word_runs("adr123-x"), vec!["adr123", "x"]);
    }

    #[test]
    fn word_runs_empty_text_is_empty() {
        assert!(word_runs("").is_empty());
    }
}
