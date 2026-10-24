//! One shared notion of "a word": a maximal run of alphanumeric
//! characters, no apostrophe or other exception. `banned::tokenize`,
//! `tells::words` and `terms::resolve`'s boundary check all go through
//! this module, so a word is the same thing everywhere. The one count that
//! differs is `word_count`, the word limit's: there an apostrophe between
//! two word characters joins its runs, so "organization's" is one word, as
//! an agent counts it.

/// Whether `c` is part of a word: plain alphanumeric, nothing else.
pub fn is_word_char(c: char) -> bool {
    c.is_alphanumeric()
}

/// Whether `a` and `b` hold the same lines once each line's trailing
/// whitespace is dropped: the one tolerance every governed-file-matches-its-
/// canonical-content check uses (the AGENTS.md brief section, the brief's
/// file under adrdom/state, and the git
/// hooks `adr install` writes and `adr check` verifies), so a file judged
/// current by one is never judged stale by the other over nothing but a
/// trailing space or a `\r` a checkout/editor left on a line.
pub fn lines_match_trimmed(a: &[&str], b: &[&str]) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b.iter())
            .all(|(x, y)| x.trim_end() == y.trim_end())
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

/// Whether `c` is an apostrophe: the plain one or the typographic one.
fn is_apostrophe(c: char) -> bool {
    c == '\'' || c == '\u{2019}'
}

/// How many words `text` holds for the word limit: the runs of `word_runs`,
/// except that an apostrophe between two word characters joins its two
/// runs into one word.
pub fn word_count(text: &str) -> usize {
    let cs: Vec<char> = text.chars().collect();
    let mut n = 0;
    let mut in_word = false;
    for (i, &c) in cs.iter().enumerate() {
        let joins = is_apostrophe(c)
            && i > 0
            && is_word_char(cs[i - 1])
            && cs.get(i + 1).is_some_and(|&d| is_word_char(d));
        if is_word_char(c) || joins {
            if !in_word {
                n += 1;
                in_word = true;
            }
        } else {
            in_word = false;
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_count_joins_a_word_across_an_apostrophe() {
        assert_eq!(word_count("the organization's quota"), 3);
        assert_eq!(word_count("the organization\u{2019}s quota"), 3);
        assert_eq!(word_count("api's"), 1);
    }

    #[test]
    fn word_count_keeps_an_apostrophe_at_a_boundary_as_a_boundary() {
        assert_eq!(word_count("'quoted' words"), 2);
        assert_eq!(word_count("ends' here"), 2);
        assert_eq!(word_count("a '' b"), 2);
        assert_eq!(word_count(""), 0);
    }

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
