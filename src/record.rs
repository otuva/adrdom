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
    /// `supersedes: ` ids, comma-separated in the source, in file order (4.14).
    pub supersedes: Vec<String>,
    pub options: Vec<OptionRec>,
    pub answer_line: Option<usize>,
    pub answer_value: Option<String>,
    /// Errors from the pre-existing parser (duplicate id, bad option/answer
    /// syntax). Unchanged in meaning and still what `check`'s `parse` check
    /// reads today.
    pub parse_errors: Vec<String>,
    /// Errors from the record-shape rules (4.9/4.7/3.4): stray lines in a
    /// body, and a non-skeleton question missing its Problem/Standard
    /// lines. Kept separate from `parse_errors` so today's `check_parse`
    /// (which reads only `parse_errors`) is unaffected by this slice; the
    /// agent that builds the new `check` behaviour is expected to fold
    /// these in under the `parse` check name, per the spec.
    #[allow(dead_code)]
    pub shape_errors: Vec<String>,
    /// True when the heading has no non-blank line before the next
    /// heading: no id, no body, never a failure (spec point 6).
    #[allow(dead_code)]
    pub is_skeleton: bool,
    /// Index into `RecordFile::groups` of the h2 group this section falls
    /// under, or `None` if it precedes the first h2 (a file-level shape
    /// error in that case).
    #[allow(dead_code)]
    pub group: Option<usize>,
    /// Line index of this section's `terms: ` line, if any.
    #[allow(dead_code)]
    pub terms_line: Option<usize>,
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

/// A `## ` heading and the questions/principles under it. File order is
/// the tier order (group 0 is lowest) — see spec point 3.
#[derive(Debug, Clone)]
pub struct Group {
    #[allow(dead_code)]
    pub heading_text: String,
    pub start: usize,
    pub end: usize,
    pub question_idx: Vec<usize>,
}

pub struct RecordFile {
    pub path: PathBuf,
    /// The path as printed: relative to the repo root with forward slashes
    /// when loaded through `load_in`, otherwise the path as given.
    pub rel: String,
    pub lines: Vec<String>,
    pub sections: Vec<Section>,
    /// The h1 topic text, or `None` if the file has no h1 (a shape error).
    #[allow(dead_code)]
    pub topic: Option<String>,
    /// [start, end) line range of the context prose: everything between
    /// the h1 and the first `## ` line (or end of file, if no h1/h2).
    #[allow(dead_code)]
    pub context_range: (usize, usize),
    #[allow(dead_code)]
    pub groups: Vec<Group>,
    /// File-level shape errors (spec point: missing/duplicate h1, text
    /// before h1, a question before the first h2, stray text between an
    /// h2 and its first h3).
    #[allow(dead_code)]
    pub shape_errors: Vec<String>,
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

/// Like `load`, with the printed path made relative to `root`.
pub fn load_in(root: &Path, path: &Path) -> std::io::Result<RecordFile> {
    let mut rf = load(path)?;
    rf.rel = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    Ok(rf)
}

fn build(path: PathBuf, rel: String, lines: Vec<String>) -> RecordFile {
    let mut sections = parse_sections(&lines);
    let (topic, context_range, groups, shape_errors) = analyze_shape(&lines, &mut sections);
    RecordFile {
        path,
        rel,
        lines,
        sections,
        topic,
        context_range,
        groups,
        shape_errors,
    }
}

pub fn save(rf: &RecordFile) -> std::io::Result<()> {
    fs::write(&rf.path, join_lines(&rf.lines))
}

fn is_question_heading(line: &str) -> bool {
    line.starts_with("### ") && line.trim_end().ends_with('?')
}

fn is_h1(line: &str) -> bool {
    line.starts_with("# ")
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

/// Everything a heading's body scan produces: the pre-existing bits
/// (id/supersedes/options/answer) plus the new shape-rule bits (terms
/// line, stray lines, Problem/Standard counts). Shared by
/// `parse_question` and `parse_principle` so both kinds get the same
/// allowed-line-types treatment (spec point 5 applies "where questions
/// are", which includes principle blocks).
struct BodyScan {
    id: Option<String>,
    id_count: usize,
    supersedes: Vec<String>,
    terms_line: Option<usize>,
    options: Vec<OptionRec>,
    answer_line: Option<usize>,
    answer_value: Option<String>,
    option_errors: Vec<String>,
    stray_errors: Vec<String>,
    problem_count: usize,
    standard_count: usize,
}

fn scan_body(lines: &[String], start: usize, end: usize) -> BodyScan {
    let mut id = None;
    let mut id_count = 0;
    let mut supersedes: Vec<String> = Vec::new();
    let mut terms_line = None;
    let mut options: Vec<OptionRec> = Vec::new();
    let mut answer_line = None;
    let mut answer_val = None;
    let mut option_errors = Vec::new();
    let mut stray_errors = Vec::new();
    let mut problem_count = 0;
    let mut standard_count = 0;
    // True while the previous non-blank line was an option or one of its
    // Why/Costs lines; a Why/Costs anywhere else is stray.
    let mut under_option = false;

    for (j, line) in lines.iter().enumerate().take(end).skip(start + 1) {
        if line.trim().is_empty() {
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
            if supersedes.is_empty() {
                supersedes = v
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            continue;
        }
        if kv(line, "terms").is_some() {
            if terms_line.is_none() {
                terms_line = Some(j);
            }
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
            }
            options.push(OptionRec {
                letter,
                sentence,
                line: j,
            });
            under_option = true;
            continue;
        }
        if line.starts_with("  Why: ") || line.starts_with("  Costs: ") {
            if under_option {
                continue;
            }
            stray_errors.push(format!("stray line {}", j + 1));
            continue;
        }
        under_option = false;
        if let Some(v) = answer_value(line) {
            answer_line = Some(j);
            answer_val = Some(v);
            continue;
        }
        stray_errors.push(format!("stray line {}", j + 1));
    }

    // Two or three options; a fourth is a shape error.
    if options.len() > 3 {
        option_errors.push(format!("{} options; at most three", options.len()));
    }

    BodyScan {
        id,
        id_count,
        supersedes,
        terms_line,
        options,
        answer_line,
        answer_value: answer_val,
        option_errors,
        stray_errors,
        problem_count,
        standard_count,
    }
}

fn is_skeleton_range(lines: &[String], start: usize, end: usize) -> bool {
    ((start + 1)..end).all(|j| lines[j].trim().is_empty())
}

fn parse_question(lines: &[String], start: usize, end: usize) -> Section {
    let heading_text = lines[start]["### ".len()..].trim_end().to_string();
    let is_skeleton = is_skeleton_range(lines, start, end);
    let scan = scan_body(lines, start, end);

    let mut parse_errors = scan.option_errors;
    if scan.id_count > 1 {
        parse_errors.push("duplicate id line".to_string());
    }
    if let Some(v) = &scan.answer_value {
        let v = v.trim();
        let valid = v.chars().count() == 1
            && scan
                .options
                .iter()
                .any(|o| o.letter == v.chars().next().unwrap());
        if !valid {
            parse_errors.push(format!("answer names missing option {}", v));
        }
    }

    let mut shape_errors = scan.stray_errors;
    if !is_skeleton {
        if scan.problem_count != 1 {
            shape_errors.push("missing Problem line".to_string());
        }
        if scan.standard_count != 1 {
            shape_errors.push("missing Standard line".to_string());
        }
    }

    Section {
        kind: Kind::Question,
        start,
        end,
        heading_text,
        id: scan.id,
        id_dup: scan.id_count > 1,
        supersedes: scan.supersedes,
        options: scan.options,
        answer_line: scan.answer_line,
        answer_value: scan.answer_value,
        parse_errors,
        shape_errors,
        is_skeleton,
        group: None,
        terms_line: scan.terms_line,
    }
}

fn parse_principle(
    lines: &[String],
    start: usize,
    end: usize,
    heading: String,
    sentence: String,
) -> Section {
    let is_skeleton = is_skeleton_range(lines, start, end);
    let scan = scan_body(lines, start, end);

    let mut parse_errors = Vec::new();
    if scan.id_count > 1 {
        parse_errors.push("duplicate id line".to_string());
    }

    let options = vec![OptionRec {
        letter: 'A',
        sentence: Some(sentence),
        line: start,
    }];

    if let Some(v) = &scan.answer_value {
        if v.trim() != "yes" {
            parse_errors.push(format!("answer names missing option {}", v.trim()));
        }
    }

    let mut shape_errors = scan.stray_errors;
    if !is_skeleton {
        if scan.problem_count != 1 {
            shape_errors.push("missing Problem line".to_string());
        }
        if scan.standard_count != 1 {
            shape_errors.push("missing Standard line".to_string());
        }
    }

    Section {
        kind: Kind::Principle,
        start,
        end,
        heading_text: heading,
        id: scan.id,
        id_dup: scan.id_count > 1,
        supersedes: scan.supersedes,
        options,
        answer_line: scan.answer_line,
        answer_value: scan.answer_value,
        parse_errors,
        shape_errors,
        is_skeleton,
        group: None,
        terms_line: scan.terms_line,
    }
}

/// Computes topic, context range, h2 groups (with sections assigned to
/// them), and file-level shape errors, given the lines and the already
/// heading-parsed `sections` (mutated in place to set `group`).
fn analyze_shape(
    lines: &[String],
    sections: &mut [Section],
) -> (Option<String>, (usize, usize), Vec<Group>, Vec<String>) {
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

    let h2_indices: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| is_h2(l))
        .map(|(i, _)| i)
        .collect();

    let context_start = h1_idx.map(|i| i + 1).unwrap_or(0);
    let context_end = h2_indices.first().copied().unwrap_or(lines.len());
    let context_range = (context_start.min(context_end), context_end);

    let mut groups: Vec<Group> = h2_indices
        .iter()
        .enumerate()
        .map(|(gi, &gstart)| {
            let gend = h2_indices.get(gi + 1).copied().unwrap_or(lines.len());
            Group {
                heading_text: lines[gstart]["## ".len()..].trim_end().to_string(),
                start: gstart,
                end: gend,
                question_idx: Vec::new(),
            }
        })
        .collect();

    for (si, sec) in sections.iter_mut().enumerate() {
        let gi = groups
            .iter()
            .position(|g| sec.start > g.start && sec.start < g.end);
        match gi {
            Some(gi) => {
                sec.group = Some(gi);
                groups[gi].question_idx.push(si);
            }
            None => {
                sec.group = None;
                shape_errors.push(format!(
                    "question before first group heading, line {}",
                    sec.start + 1
                ));
            }
        }
    }

    for g in &groups {
        let first_q_start = g
            .question_idx
            .iter()
            .map(|&si| sections[si].start)
            .min()
            .unwrap_or(g.end);
        for j in (g.start + 1)..first_q_start {
            if !lines[j].trim().is_empty() {
                shape_errors.push(format!("stray line {}", j + 1));
            }
        }
    }

    (topic, context_range, groups, shape_errors)
}

/// sha256 of the h2 and h3/principle heading lines (trimmed text incl.
/// the `## `/`### `/`**Pn.` prefix), joined with `\n`, in file order
/// (spec point 8). Changes on an added, reordered, or renamed heading;
/// unaffected by body edits.
#[allow(dead_code)]
pub fn plan_hash(rf: &RecordFile) -> String {
    let mut items: Vec<(usize, &str)> = Vec::new();
    for g in &rf.groups {
        items.push((g.start, rf.lines[g.start].trim_end()));
    }
    for s in &rf.sections {
        items.push((s.start, rf.lines[s.start].trim_end()));
    }
    items.sort_by_key(|(i, _)| *i);
    let text = items
        .into_iter()
        .map(|(_, t)| t)
        .collect::<Vec<_>>()
        .join("\n");
    sha256_hex(&text)
}

/// sha256 of the context prose lines with leading/trailing blank lines
/// trimmed, joined with `\n`; the empty string hashes when there is no
/// prose (spec point 9).
#[allow(dead_code)]
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
    let text = lines[s..e]
        .iter()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n");
    sha256_hex(&text)
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
            if let Some(v) = line.strip_prefix("  Why: ") {
                if why.is_none() {
                    why = Some(v.trim().to_string());
                }
                continue;
            }
            if let Some(v) = line.strip_prefix("  Costs: ") {
                if costs.is_none() {
                    costs = Some(v.trim().to_string());
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

    const SHAPED: &str = "# Topic\n\ncontext prose line one.\ncontext prose line two.\n\n## Group A\n\n### 1.1 First question?\n\nProblem: p.\n\nStandard: s.\n\n- A) Do the thing.\n  Why: reasons.\n";

    #[test]
    fn h1_required_and_first() {
        let rf = rf_from("no h1 here\n\n## Group\n\n### 1.1 Q?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n");
        assert!(rf.topic.is_none());
        assert!(rf.shape_errors.iter().any(|e| e.contains("missing h1")));

        let rf2 = rf_from("stray text\n# Topic\n\n## Group\n");
        assert_eq!(rf2.topic.as_deref(), Some("Topic"));
        assert!(rf2.shape_errors.iter().any(|e| e.contains("text before h1")));

        let rf3 = rf_from("# Topic\n\n# Second Topic\n\n## Group\n");
        assert!(rf3.shape_errors.iter().any(|e| e.contains("duplicate h1")));

        let rf4 = rf_from(SHAPED);
        assert_eq!(rf4.topic.as_deref(), Some("Topic"));
        assert!(rf4.shape_errors.is_empty(), "{:?}", rf4.shape_errors);
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
        let rf = rf_from("# Topic\n\n## Group\n\n### 1.1 Skeleton?\n\n### 1.2 Real question?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n");
        let skeleton = &rf.sections[0];
        assert!(skeleton.is_skeleton);
        assert!(skeleton.id.is_none());
        assert!(skeleton.shape_errors.is_empty(), "{:?}", skeleton.shape_errors);

        let real = &rf.sections[1];
        assert!(!real.is_skeleton);
    }

    #[test]
    fn stray_line_in_body_is_a_shape_error_naming_the_line() {
        let content = "# Topic\n\n## Group\n\n### 1.1 Q?\n\nProblem: p.\n\nStandard: s.\n\nthis line is not allowed\n\n- A) x.\n";
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

    #[test]
    fn missing_problem_or_standard_only_fails_non_skeleton_questions() {
        let rf = rf_from("# Topic\n\n## Group\n\n### 1.1 Skeleton?\n\n### 1.2 Has a body but no Problem/Standard?\n\n- A) x.\n");
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
    fn plan_hash_changes_on_heading_add_reorder_rename_not_on_body_edit() {
        let base = rf_from(SHAPED);
        let base_hash = plan_hash(&base);

        // Body-only edit: unchanged.
        let body_edited = SHAPED.replace("Problem: p.", "Problem: a very different problem.");
        let rf_body = rf_from(&body_edited);
        assert_eq!(base_hash, plan_hash(&rf_body), "body edit must not move plan hash");

        // Heading added.
        let added = format!("{}\n### 1.2 A second question?\n\nProblem: p2.\n\nStandard: s2.\n\n- A) y.\n", SHAPED);
        let rf_added = rf_from(&added);
        assert_ne!(base_hash, plan_hash(&rf_added), "adding a heading must move plan hash");

        // Heading renamed.
        let renamed = SHAPED.replace("### 1.1 First question?", "### 1.1 First question, renamed?");
        let rf_renamed = rf_from(&renamed);
        assert_ne!(base_hash, plan_hash(&rf_renamed), "renaming a heading must move plan hash");

        // Headings reordered: two groups, each with one question, swapped.
        let two_groups = "# Topic\n\n## Group A\n\n### 1.1 Question A?\n\nProblem: pa.\n\nStandard: sa.\n\n- A) x.\n\n## Group B\n\n### 1.2 Question B?\n\nProblem: pb.\n\nStandard: sb.\n\n- A) y.\n";
        let swapped = "# Topic\n\n## Group B\n\n### 1.2 Question B?\n\nProblem: pb.\n\nStandard: sb.\n\n- A) y.\n\n## Group A\n\n### 1.1 Question A?\n\nProblem: pa.\n\nStandard: sa.\n\n- A) x.\n";
        let rf_two = rf_from(two_groups);
        let rf_swapped = rf_from(swapped);
        assert_ne!(
            plan_hash(&rf_two),
            plan_hash(&rf_swapped),
            "reordering headings must move plan hash"
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

    #[test]
    fn supersedes_line_parses_multiple_comma_separated_ids() {
        let rf = rf_from("# Topic\n\n## Group\n\n### 1.1 Q?\n\nid: new-id\n\nsupersedes: old-1, old-2\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n");
        assert_eq!(
            rf.sections[0].supersedes,
            vec!["old-1".to_string(), "old-2".to_string()]
        );
    }

    #[test]
    fn an_option_decides_exactly_its_first_sentence() {
        assert_eq!(
            extract_sentence("Ties live in code. Because tests move."),
            Some("Ties live in code.".to_string())
        );
        assert_eq!(extract_sentence("Ties live in code"), None);
    }

    #[test]
    fn a_lean_line_inside_a_question_is_a_shape_error() {
        let rf = rf_from(
            "# T\n\n## G\n\n### Q?\n\nProblem: p.\nStandard: s.\nLean: A.\n\n- A) Yes.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n",
        );
        assert!(rf.sections[0]
            .shape_errors
            .iter()
            .any(|e| e.starts_with("stray line")));
    }

    #[test]
    fn a_why_line_before_any_option_is_stray() {
        let rf = rf_from(
            "# T\n\n## G\n\n### Q?\n\nProblem: p.\nStandard: s.\n  Why: early.\n\n- A) Yes.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n",
        );
        assert!(rf.sections[0]
            .shape_errors
            .iter()
            .any(|e| e == "stray line 9"));
        let rf = rf_from(
            "# T\n\n## G\n\n### Q?\n\nProblem: p.\nStandard: s.\n\n- A) Yes.\n  Why: w.\n  Costs: c.\n- B) No.\n  Why: w.\n  Costs: c.\n\n**Answer:** A\n  Costs: late.\n",
        );
        assert!(rf.sections[0]
            .shape_errors
            .iter()
            .any(|e| e.starts_with("stray line")));
    }

    #[test]
    fn a_fourth_option_is_a_shape_error() {
        let rf = rf_from(
            "# T\n\n## G\n\n### Q?\n\nProblem: p.\nStandard: s.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n- C) Three.\n  Why: w.\n  Costs: c.\n- D) Four.\n  Why: w.\n  Costs: c.\n",
        );
        assert!(rf.sections[0]
            .parse_errors
            .iter()
            .any(|e| e == "4 options; at most three"));
        let ok = rf_from(
            "# T\n\n## G\n\n### Q?\n\nProblem: p.\nStandard: s.\n\n- A) One.\n  Why: w.\n  Costs: c.\n- B) Two.\n  Why: w.\n  Costs: c.\n- C) Three.\n  Why: w.\n  Costs: c.\n",
        );
        assert!(ok.sections[0].parse_errors.is_empty());
    }
}
