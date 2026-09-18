use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Question,
    Principle,
}

#[derive(Debug, Clone)]
pub struct OptionRec {
    pub letter: char,
    pub sentence: Option<String>,
    #[allow(dead_code)]
    pub line: usize,
}

#[derive(Debug, Clone)]
pub struct Section {
    pub kind: Kind,
    pub start: usize,
    pub end: usize,
    pub heading_text: String,
    pub id: Option<String>,
    #[allow(dead_code)]
    pub id_dup: bool,
    pub supersedes: Option<String>,
    pub options: Vec<OptionRec>,
    pub answer_line: Option<usize>,
    pub answer_value: Option<String>,
    pub parse_errors: Vec<String>,
}

impl Section {
    pub fn answer_letter(&self) -> Option<char> {
        let v = self.answer_value.as_ref()?.trim();
        match self.kind {
            Kind::Question => {
                if v.chars().count() == 1 {
                    let c = v.chars().next().unwrap();
                    if self.options.iter().any(|o| o.letter == c) {
                        return Some(c);
                    }
                }
                None
            }
            Kind::Principle => {
                if v == "yes" {
                    Some('A')
                } else {
                    None
                }
            }
        }
    }

    pub fn chosen_sentence(&self) -> Option<&str> {
        let c = self.answer_letter()?;
        self.options
            .iter()
            .find(|o| o.letter == c)
            .and_then(|o| o.sentence.as_deref())
    }
}

pub struct RecordFile {
    pub path: PathBuf,
    pub lines: Vec<String>,
    pub sections: Vec<Section>,
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
    let sections = parse_sections(&lines);
    Ok(RecordFile {
        path: path.to_path_buf(),
        lines,
        sections,
    })
}

pub fn save(rf: &RecordFile) -> std::io::Result<()> {
    fs::write(&rf.path, join_lines(&rf.lines))
}

fn is_question_heading(line: &str) -> bool {
    line.starts_with("### ") && line.trim_end().ends_with('?')
}

fn is_h2(line: &str) -> bool {
    line.starts_with("## ")
}

fn parse_principle_heading(line: &str) -> Option<(String, String)> {
    let l = line.trim_end();
    if !l.starts_with("**P") {
        return None;
    }
    let rest = &l[3..];
    let digits_len = rest.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits_len == 0 {
        return None;
    }
    let digits = &rest[..digits_len];
    let after_digits = &rest[digits_len..];
    if !after_digits.starts_with(". ") {
        return None;
    }
    let after_dot = &after_digits[2..];
    if after_dot.len() < 2 || !after_dot.ends_with("**") {
        return None;
    }
    let sentence = &after_dot[..after_dot.len() - 2];
    if sentence.is_empty() {
        return None;
    }
    Some((format!("P{}", digits), sentence.to_string()))
}

fn parse_option_line(line: &str) -> Option<(char, String)> {
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

fn kv(line: &str, key: &str) -> Option<String> {
    let prefix = format!("{}: ", key);
    line.strip_prefix(&prefix).map(|v| v.trim().to_string())
}

fn answer_value(line: &str) -> Option<String> {
    line.strip_prefix("**Answer:** ")
        .map(|v| v.trim().to_string())
}

pub fn parse_sections(lines: &[String]) -> Vec<Section> {
    let mut sections = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = &lines[i];
        if is_question_heading(line) {
            let start = i;
            let end = find_end(lines, i + 1);
            sections.push(parse_question(lines, start, end));
            i = end;
        } else if let Some((heading, sentence)) = parse_principle_heading(line) {
            let start = i;
            let end = find_end(lines, i + 1);
            sections.push(parse_principle(lines, start, end, heading, sentence));
            i = end;
        } else {
            i += 1;
        }
    }
    sections
}

fn find_end(lines: &[String], from: usize) -> usize {
    for (j, line) in lines.iter().enumerate().skip(from) {
        if is_question_heading(line) || is_h2(line) || parse_principle_heading(line).is_some() {
            return j;
        }
    }
    lines.len()
}

fn parse_question(lines: &[String], start: usize, end: usize) -> Section {
    let heading_text = lines[start]["### ".len()..].trim_end().to_string();
    let mut id = None;
    let mut id_count = 0;
    let mut supersedes = None;
    let mut options: Vec<OptionRec> = Vec::new();
    let mut answer_line = None;
    let mut answer_val = None;
    let mut parse_errors = Vec::new();

    for (j, line) in lines.iter().enumerate().take(end).skip(start + 1) {
        if let Some(v) = kv(line, "id") {
            if id_count == 0 {
                id = Some(v);
            }
            id_count += 1;
        } else if let Some(v) = kv(line, "supersedes") {
            if supersedes.is_none() {
                supersedes = Some(v);
            }
        } else if let Some((letter, rest)) = parse_option_line(line) {
            let sentence = extract_sentence(&rest);
            if sentence.is_none() {
                parse_errors.push(format!("option {} has no decision sentence", letter));
            }
            options.push(OptionRec {
                letter,
                sentence,
                line: j,
            });
        } else if let Some(v) = answer_value(line) {
            answer_line = Some(j);
            answer_val = Some(v);
        }
    }

    if id_count > 1 {
        parse_errors.push("duplicate id line".to_string());
    }

    if let Some(v) = &answer_val {
        let v = v.trim();
        let valid = v.chars().count() == 1
            && options
                .iter()
                .any(|o| o.letter == v.chars().next().unwrap());
        if !valid {
            parse_errors.push(format!("answer names missing option {}", v));
        }
    }

    Section {
        kind: Kind::Question,
        start,
        end,
        heading_text,
        id,
        id_dup: id_count > 1,
        supersedes,
        options,
        answer_line,
        answer_value: answer_val,
        parse_errors,
    }
}

fn parse_principle(
    lines: &[String],
    start: usize,
    end: usize,
    heading: String,
    sentence: String,
) -> Section {
    let mut id = None;
    let mut id_count = 0;
    let mut answer_line = None;
    let mut answer_val = None;
    let mut parse_errors = Vec::new();

    for (j, line) in lines.iter().enumerate().take(end).skip(start + 1) {
        if let Some(v) = kv(line, "id") {
            if id_count == 0 {
                id = Some(v);
            }
            id_count += 1;
        } else if let Some(v) = answer_value(line) {
            answer_line = Some(j);
            answer_val = Some(v);
        }
    }

    if id_count > 1 {
        parse_errors.push("duplicate id line".to_string());
    }

    let options = vec![OptionRec {
        letter: 'A',
        sentence: Some(sentence),
        line: start,
    }];

    if let Some(v) = &answer_val {
        if v.trim() != "yes" {
            parse_errors.push(format!("answer names missing option {}", v.trim()));
        }
    }

    Section {
        kind: Kind::Principle,
        start,
        end,
        heading_text: heading,
        id,
        id_dup: id_count > 1,
        supersedes: None,
        options,
        answer_line,
        answer_value: answer_val,
        parse_errors,
    }
}

/// Returns [start, hash_end) — the line range whose joined bytes are hashed.
pub fn hash_range(lines: &[String], sec: &Section) -> (usize, usize) {
    // The boundary to search backward from for the last non-blank line:
    // just before the Answer line when there is one, otherwise the end of
    // the section. Trailing blank lines are never hashed, so inserting the
    // "blank line before the Answer line" (as `adr lock` does) can never
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

pub fn section_hash(lines: &[String], sec: &Section) -> String {
    let (s, e) = hash_range(lines, sec);
    let text = lines[s..e].join("\n");
    sha256_hex(&text)
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

pub fn sha256_hex(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hex(&hasher.finalize())
}
