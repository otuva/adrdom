use crate::record::{normalize, sha256_hex};
use std::collections::{BTreeSet, HashMap};
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

pub struct TermsFile {
    pub path: PathBuf,
    pub lines: Vec<String>,
    pub terms: Vec<Term>,
    pub errors: Vec<String>,
}

fn kv(line: &str, key: &str) -> Option<String> {
    let prefix = format!("{}:", key);
    line.strip_prefix(&prefix).map(|v| v.trim().to_string())
}

fn split_csv(s: &str) -> Vec<String> {
    s.split(',')
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .collect()
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

pub fn file_hash(tf: &TermsFile) -> String {
    let text = tf
        .lines
        .iter()
        .map(|l| l.trim_end())
        .collect::<Vec<_>>()
        .join("\n");
    sha256_hex(&normalize(&text))
}

pub fn parse(lines: &[String]) -> TermsFile {
    let mut errors = Vec::new();
    let mut terms: Vec<Term> = Vec::new();

    let h1_lines: Vec<usize> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| l.trim_end() == "# Terms")
        .map(|(i, _)| i)
        .collect();
    if h1_lines.is_empty() {
        errors.push("line 1: missing h1 \"# Terms\"".to_string());
    } else {
        for &i in h1_lines.iter().skip(1) {
            errors.push(format!("line {}: duplicate h1 \"# Terms\"", i + 1));
        }
    }

    let mut heading_idxs: Vec<usize> = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        if l.starts_with("## ") && !l.starts_with("### ") {
            heading_idxs.push(i);
        }
    }

    let first_heading = heading_idxs.first().copied().unwrap_or(lines.len());
    for (i, l) in lines.iter().enumerate().take(first_heading) {
        if l.trim().is_empty() {
            continue;
        }
        if h1_lines.contains(&i) {
            continue;
        }
        errors.push(format!("line {}: unknown line", i + 1));
    }

    for (hi, &start) in heading_idxs.iter().enumerate() {
        let end = heading_idxs.get(hi + 1).copied().unwrap_or(lines.len());
        let name = lines[start]["## ".len()..].trim_end().to_string();
        if name.trim().is_empty() {
            errors.push(format!("line {}: empty name", start + 1));
        }

        let mut id: Option<String> = None;
        let mut id_line: Option<usize> = None;
        let mut definition: Option<String> = None;
        let mut aliases: Option<String> = None;
        let mut terms_line: Option<String> = None;
        let mut terms_line_idx: Option<usize> = None;
        let mut retired = false;
        let mut retired_seen = false;

        for (j, l) in lines.iter().enumerate().take(end).skip(start + 1) {
            if l.trim().is_empty() {
                continue;
            }
            if l.trim_end() == "# Terms" {
                // Already reported as a duplicate h1 above.
                continue;
            }
            if kv(l, "id").is_some() {
                if id.is_none() {
                    id = kv(l, "id");
                    id_line = Some(j);
                } else {
                    errors.push(format!("line {}: duplicate field \"id\"", j + 1));
                }
            } else if kv(l, "definition").is_some() {
                if definition.is_none() {
                    definition = kv(l, "definition");
                } else {
                    errors.push(format!("line {}: duplicate field \"definition\"", j + 1));
                }
            } else if kv(l, "aliases").is_some() {
                if aliases.is_none() {
                    aliases = kv(l, "aliases");
                } else {
                    errors.push(format!("line {}: duplicate field \"aliases\"", j + 1));
                }
            } else if kv(l, "terms").is_some() {
                if terms_line.is_none() {
                    terms_line = Some(l.trim_end().to_string());
                    terms_line_idx = Some(j);
                } else {
                    errors.push(format!("line {}: duplicate field \"terms\"", j + 1));
                }
            } else if let Some(v) = kv(l, "retired") {
                if retired_seen {
                    errors.push(format!("line {}: duplicate field \"retired\"", j + 1));
                } else {
                    retired_seen = true;
                    if v == "yes" {
                        retired = true;
                    } else {
                        errors.push(format!("line {}: unknown line", j + 1));
                    }
                }
            } else {
                errors.push(format!("line {}: unknown line", j + 1));
            }
        }

        if id.is_none() {
            errors.push(format!(
                "line {}: missing id; owner runs adr approve adrdom/terms.md",
                start + 1
            ));
        }
        if definition.is_none() {
            errors.push(format!("line {}: missing field \"definition\"", start + 1));
        }
        if aliases.is_none() {
            errors.push(format!("line {}: missing field \"aliases\"", start + 1));
        }

        terms.push(Term {
            name,
            id,
            definition: definition.unwrap_or_default(),
            aliases: split_csv(aliases.as_deref().unwrap_or("")),
            retired,
            start_line: start + 1,
            id_line: id_line.map(|x| x + 1),
            terms_line,
            terms_line_idx: terms_line_idx.map(|x| x + 1),
        });
    }

    let mut seen_ids: HashMap<String, usize> = HashMap::new();
    for t in &terms {
        if let Some(id) = &t.id {
            let line = t.id_line.unwrap_or(t.start_line);
            if let Some(&prev_line) = seen_ids.get(id) {
                errors.push(format!(
                    "line {}: duplicate id \"{}\" (also used at line {})",
                    line, id, prev_line
                ));
            } else {
                seen_ids.insert(id.clone(), line);
            }
        }
    }

    errors.extend(duplicate_word_errors(&terms));

    TermsFile {
        path: PathBuf::new(),
        lines: lines.to_vec(),
        terms,
        errors,
    }
}

/// A retired entry's name and aliases are ignored here, so a current term
/// may take a retired term's word (9.3 B); two current entries sharing a
/// word still refuse.
fn duplicate_word_errors(terms: &[Term]) -> Vec<String> {
    let mut occ: HashMap<String, Vec<usize>> = HashMap::new();
    for (idx, t) in terms.iter().enumerate() {
        if t.retired {
            continue;
        }
        let mut namealias = vec![t.name.clone()];
        namealias.extend(t.aliases.iter().cloned());
        for w in namealias {
            let wl = w.trim().to_lowercase();
            if wl.is_empty() {
                continue;
            }
            occ.entry(wl).or_default().push(idx);
        }
    }

    let mut errors = Vec::new();
    let mut reported: BTreeSet<(String, usize, usize)> = BTreeSet::new();
    let mut words: Vec<&String> = occ.keys().collect();
    words.sort();
    for word in words {
        let mut entries = occ[word].clone();
        entries.sort_unstable();
        entries.dedup();

        if entries.len() > 1 {
            for wi in 0..entries.len() {
                for wj in (wi + 1)..entries.len() {
                    let a = entries[wi];
                    let b = entries[wj];
                    let key = (word.clone(), a.min(b), a.max(b));
                    if reported.insert(key) {
                        errors.push(format!(
                            "line {}: word \"{}\" used in both \"{}\" (line {}) and \"{}\" (line {})",
                            terms[a].start_line.min(terms[b].start_line),
                            word,
                            terms[a].name,
                            terms[a].start_line,
                            terms[b].name,
                            terms[b].start_line,
                        ));
                    }
                }
            }
        }
    }
    errors
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

fn is_boundary_ok(chars: &[char], start: usize, len: usize) -> bool {
    let before_ok = start == 0 || !chars[start - 1].is_alphanumeric();
    let after_ok = start + len == chars.len() || !chars[start + len].is_alphanumeric();
    before_ok && after_ok
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum MatchKind {
    Exact,
    Es,
    S,
}

/// Every name and alias in `terms` (and its "s"/"es" plural forms),
/// lowercased, mapped to the id of the term that owns it. Built with the
/// same priority `resolve` uses when a string is a candidate match for more
/// than one term: exact beats the "es" plural beats the "s" plural, and
/// among candidates of equal priority the first one built (terms in
/// `terms` order, name before its aliases) wins. Terms without an id, and
/// retired terms, are skipped (9.3 B): a lookup never lands on a retired
/// term, so a new pairing sees current terms only.
///
/// A single lookup against this map answers whether a whole word/phrase
/// still resolves to a given id exactly as `resolve(word, terms)` would
/// (a stored pairing is always the whole matched text, so only a
/// full-string candidate can produce it); built once and reused across
/// many words instead of rebuilding `resolve`'s phrase list per word.
pub fn phrase_lookup(terms: &[Term]) -> HashMap<String, String> {
    let mut map: HashMap<String, (String, MatchKind)> = HashMap::new();
    for t in terms {
        if t.retired {
            continue;
        }
        let Some(id) = &t.id else { continue };
        let mut names = vec![t.name.clone()];
        names.extend(t.aliases.iter().cloned());
        for n in names {
            let nl = n.trim().to_lowercase();
            if nl.is_empty() {
                continue;
            }
            for (key, kind) in [
                (nl.clone(), MatchKind::Exact),
                (format!("{}es", nl), MatchKind::Es),
                (format!("{}s", nl), MatchKind::S),
            ] {
                let should_insert = match map.get(&key) {
                    None => true,
                    Some((_, existing_kind)) => kind < *existing_kind,
                };
                if should_insert {
                    map.insert(key, (id.clone(), kind));
                }
            }
        }
    }
    map.into_iter().map(|(k, (id, _))| (k, id)).collect()
}

/// Whole-word/phrase, case-insensitive, longest-phrase-first, non-overlapping
/// resolution of every term name/alias in `text`. Retired terms are never
/// matched (9.3 B): every new pairing sees current terms only, so a word
/// that only a retired term owns resolves to nothing here (a current term
/// is free to take that same name or alias). Terms without an id are
/// skipped. Returned pairs are unique and sorted by matched word.
///
/// A word also matches a name or alias when it equals that name/alias with a
/// trailing "s" or "es" appended (simple plural), tried in that order after
/// the exact match; the matched text is stored as-is (e.g. `decisions=<id>`).
/// Irregular plurals are ordinary aliases and match exactly.
pub fn resolve(text: &str, terms: &[Term]) -> Vec<(String, String)> {
    let mut phrases: Vec<(Vec<char>, String, MatchKind)> = Vec::new();
    for t in terms {
        if t.retired {
            continue;
        }
        let id = match &t.id {
            Some(i) => i.clone(),
            None => continue,
        };
        let mut names = vec![t.name.clone()];
        names.extend(t.aliases.iter().cloned());
        for n in names {
            let nl = n.trim().to_lowercase();
            if nl.is_empty() {
                continue;
            }
            phrases.push((nl.chars().collect(), id.clone(), MatchKind::Exact));
            phrases.push((format!("{}es", nl).chars().collect(), id.clone(), MatchKind::Es));
            phrases.push((format!("{}s", nl).chars().collect(), id.clone(), MatchKind::S));
        }
    }
    // Longest phrase first; then exact over "es"-plural over "s"-plural;
    // ties broken lexicographically for determinism.
    phrases.sort_by(|a, b| {
        b.0.len().cmp(&a.0.len()).then_with(|| a.2.cmp(&b.2)).then_with(|| {
            let sa: String = a.0.iter().collect();
            let sb: String = b.0.iter().collect();
            sa.cmp(&sb)
        })
    });

    let lower_text = text.to_lowercase();
    let chars: Vec<char> = lower_text.chars().collect();
    let n = chars.len();
    let mut results: BTreeSet<(String, String)> = BTreeSet::new();
    let mut i = 0;
    while i < n {
        let mut matched_len = 0;
        for (phrase, id, _kind) in &phrases {
            let plen = phrase.len();
            if plen == 0 || i + plen > n {
                continue;
            }
            if chars[i..i + plen] == phrase[..] && is_boundary_ok(&chars, i, plen) {
                let word: String = chars[i..i + plen].iter().collect();
                results.insert((word, id.clone()));
                matched_len = plen;
                break;
            }
        }
        if matched_len > 0 {
            i += matched_len;
        } else {
            i += 1;
        }
    }
    results.into_iter().collect()
}

/// Resolves term `idx`'s definition text against every current term in
/// `tf` (including itself, so a self-pairing can be detected; a retired
/// `tf.terms[idx]` never matches, since `resolve` skips retired targets).
///
/// Used by `cycles` below and by approve's circular-definition check.
pub fn definition_pairs(tf: &TermsFile, idx: usize) -> Vec<(String, String)> {
    resolve(&tf.terms[idx].definition, &tf.terms)
}

/// One cycle per strongly connected component (Tarjan's algorithm) of the
/// graph where a term points at every term its definition pairs (by
/// id); a component with only one node and no self-loop isn't a cycle and
/// is skipped. Each reported cycle is one simple cycle found by a DFS
/// restricted to that component, as the list of term names in traversal
/// order starting from the component's lowest-index member; a self-cycle
/// is a list of one name. Linear in the number of terms and pairings,
/// unlike enumerating every simple cycle (which is factorial in a dense
/// graph).
///
/// Used by approve's and check's circular-definition refusal.
pub fn cycles(tf: &TermsFile) -> Vec<Vec<String>> {
    let n = tf.terms.len();
    let mut id_to_idx: HashMap<&str, usize> = HashMap::new();
    for (i, t) in tf.terms.iter().enumerate() {
        if let Some(id) = &t.id {
            id_to_idx.insert(id.as_str(), i);
        }
    }

    let mut adj: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); n];
    for i in 0..n {
        if tf.terms[i].id.is_none() {
            continue;
        }
        for (_, id) in definition_pairs(tf, i) {
            if let Some(&j) = id_to_idx.get(id.as_str()) {
                adj[i].insert(j);
            }
        }
    }

    let mut out: Vec<(usize, Vec<String>)> = Vec::new();
    for scc in tarjan_scc(&adj) {
        let has_self_loop = scc.len() == 1 && adj[scc[0]].contains(&scc[0]);
        if scc.len() <= 1 && !has_self_loop {
            continue;
        }
        let start = *scc.iter().min().expect("scc is non-empty");
        let cycle_idx = find_cycle_in_scc(&scc, &adj, start);
        let names = cycle_idx.iter().map(|&i| tf.terms[i].name.clone()).collect();
        out.push((start, names));
    }
    out.sort_by_key(|(start, _)| *start);
    out.into_iter().map(|(_, names)| names).collect()
}

/// Tarjan's strongly connected components algorithm over `adj` (an
/// adjacency list indexed by node). Returns each component as the list of
/// its member indices, in no particular order.
fn tarjan_scc(adj: &[BTreeSet<usize>]) -> Vec<Vec<usize>> {
    let n = adj.len();
    let mut index_counter = 0usize;
    let mut indices: Vec<Option<usize>> = vec![None; n];
    let mut lowlink: Vec<usize> = vec![0; n];
    let mut on_stack: Vec<bool> = vec![false; n];
    let mut stack: Vec<usize> = Vec::new();
    let mut sccs: Vec<Vec<usize>> = Vec::new();

    for u in 0..n {
        if indices[u].is_none() {
            tarjan_visit(
                u,
                adj,
                &mut index_counter,
                &mut indices,
                &mut lowlink,
                &mut on_stack,
                &mut stack,
                &mut sccs,
            );
        }
    }
    sccs
}

fn tarjan_visit(
    u: usize,
    adj: &[BTreeSet<usize>],
    index_counter: &mut usize,
    indices: &mut [Option<usize>],
    lowlink: &mut [usize],
    on_stack: &mut [bool],
    stack: &mut Vec<usize>,
    sccs: &mut Vec<Vec<usize>>,
) {
    indices[u] = Some(*index_counter);
    lowlink[u] = *index_counter;
    *index_counter += 1;
    stack.push(u);
    on_stack[u] = true;

    for &v in &adj[u] {
        if indices[v].is_none() {
            tarjan_visit(v, adj, index_counter, indices, lowlink, on_stack, stack, sccs);
            lowlink[u] = lowlink[u].min(lowlink[v]);
        } else if on_stack[v] {
            lowlink[u] = lowlink[u].min(indices[v].expect("visited node has an index"));
        }
    }

    if lowlink[u] == indices[u].expect("u was just visited") {
        let mut component = Vec::new();
        loop {
            let w = stack.pop().expect("u is still on the stack");
            on_stack[w] = false;
            component.push(w);
            if w == u {
                break;
            }
        }
        sccs.push(component);
    }
}

/// One simple cycle inside the strongly connected component `scc`, found by
/// a DFS restricted to its members, starting from `start`. Every member of
/// `scc` with more than one node (or a self-loop) reaches such a cycle by
/// the component's definition, so this always finds one.
fn find_cycle_in_scc(scc: &[usize], adj: &[BTreeSet<usize>], start: usize) -> Vec<usize> {
    let members: BTreeSet<usize> = scc.iter().copied().collect();
    let mut visited: BTreeSet<usize> = BTreeSet::new();
    let mut on_stack: Vec<usize> = Vec::new();
    let mut on_stack_set: BTreeSet<usize> = BTreeSet::new();
    let mut found: Option<Vec<usize>> = None;
    find_cycle_dfs(
        start,
        &members,
        adj,
        &mut visited,
        &mut on_stack,
        &mut on_stack_set,
        &mut found,
    );
    found.expect("a component with >1 node or a self-loop contains a cycle")
}

fn find_cycle_dfs(
    u: usize,
    members: &BTreeSet<usize>,
    adj: &[BTreeSet<usize>],
    visited: &mut BTreeSet<usize>,
    on_stack: &mut Vec<usize>,
    on_stack_set: &mut BTreeSet<usize>,
    found: &mut Option<Vec<usize>>,
) {
    if found.is_some() {
        return;
    }
    visited.insert(u);
    on_stack.push(u);
    on_stack_set.insert(u);

    for &v in &adj[u] {
        if found.is_some() {
            break;
        }
        if !members.contains(&v) {
            continue;
        }
        if on_stack_set.contains(&v) {
            let pos = on_stack.iter().position(|&x| x == v).expect("v is on the stack");
            *found = Some(on_stack[pos..].to_vec());
            break;
        } else if !visited.contains(&v) {
            find_cycle_dfs(v, members, adj, visited, on_stack, on_stack_set, found);
        }
    }

    on_stack.pop();
    on_stack_set.remove(&u);
}

/// Inserts, replaces, or removes each term's `terms:` line right after its
/// `id:` line, in `lines` (a working copy of `tf.lines`). `pairs_per_entry`
/// gives each term's pairs in the same order as `tf.terms`; an empty list
/// omits the line. Callers write `lines` to disk.
///
/// Used by approve's write step.
pub fn write_terms_lines(
    lines: &mut Vec<String>,
    tf: &TermsFile,
    pairs_per_entry: &[Vec<(String, String)>],
) {
    enum Op {
        Remove,
        Insert(String),
    }

    let mut ops: Vec<(usize, u8, Op)> = Vec::new();
    for (i, t) in tf.terms.iter().enumerate() {
        let pairs = pairs_per_entry.get(i).map(|v| v.as_slice()).unwrap_or(&[]);
        let new_line = format_terms_line(pairs);

        if let Some(idx) = t.terms_line_idx {
            // 1-based line number -> 0-based index.
            ops.push((idx - 1, 0, Op::Remove));
        }
        if let (Some(new_line), Some(id_line)) = (new_line, t.id_line) {
            // 1-based id line number is also the 0-based "insert right after" index.
            ops.push((id_line, 1, Op::Insert(new_line)));
        }
    }

    // Bottom-to-top (position descending) so earlier edits don't shift the
    // indices later edits still need; a remove is applied before an insert
    // at the same position so a "replace in place" lands correctly.
    ops.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    for (pos, _, op) in ops {
        match op {
            Op::Remove => {
                if pos < lines.len() {
                    lines.remove(pos);
                }
            }
            Op::Insert(s) => {
                let pos = pos.min(lines.len());
                lines.insert(pos, s);
            }
        }
    }
}

pub fn format_terms_line(pairs: &[(String, String)]) -> Option<String> {
    if pairs.is_empty() {
        return None;
    }
    let body = pairs
        .iter()
        .map(|(w, id)| format!("{}={}", w, id))
        .collect::<Vec<_>>()
        .join(", ");
    Some(format!("terms: {}", body))
}

pub fn parse_terms_line(line: &str) -> Option<Vec<(String, String)>> {
    let rest = line.strip_prefix("terms: ")?;
    if rest.trim().is_empty() {
        return None;
    }
    let mut out = Vec::new();
    for part in rest.split(", ") {
        let (w, id) = part.split_once('=')?;
        out.push((w.to_string(), id.to_string()));
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn lines_of(s: &str) -> Vec<String> {
        crate::record::split_lines(s)
    }

    fn well_formed() -> String {
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
    fn well_formed_file_parses_with_no_errors() {
        let tf = parse(&lines_of(&well_formed()));
        assert!(tf.errors.is_empty(), "unexpected errors: {:?}", tf.errors);
        assert_eq!(tf.terms.len(), 2);
        assert_eq!(tf.terms[0].name, "decision");
        assert_eq!(
            tf.terms[0].id.as_deref(),
            Some("11111111-1111-1111-1111-111111111111")
        );
        assert_eq!(tf.terms[0].aliases, vec!["choice".to_string()]);
        assert!(!tf.terms[0].retired);
        assert!(tf.terms[1].aliases.is_empty());
    }

    #[test]
    fn retired_line_is_recognized() {
        let mut s = well_formed();
        s = s.replacen("aliases: choice\n", "aliases: choice\nretired: yes\n", 1);
        let tf = parse(&lines_of(&s));
        assert!(tf.errors.is_empty(), "unexpected errors: {:?}", tf.errors);
        assert!(tf.terms[0].retired);
    }

    #[test]
    fn missing_h1_is_a_shape_error() {
        let s = well_formed().replacen("# Terms\n", "", 1);
        let tf = parse(&lines_of(&s));
        assert!(tf.errors.iter().any(|e| e.contains("missing h1")));
    }

    #[test]
    fn duplicate_h1_is_a_shape_error() {
        let s = format!("# Terms\n{}", well_formed());
        let tf = parse(&lines_of(&s));
        assert!(tf.errors.iter().any(|e| e.contains("duplicate h1")));
    }

    #[test]
    fn missing_field_is_a_shape_error() {
        let s = well_formed().replacen("definition: A choice answered by the owner.\n", "", 1);
        let tf = parse(&lines_of(&s));
        assert!(tf
            .errors
            .iter()
            .any(|e| e.contains("missing field \"definition\"")));
    }

    #[test]
    fn unknown_line_is_a_shape_error() {
        let s = well_formed().replacen("aliases: choice\n", "aliases: choice\nnotes: extra\n", 1);
        let tf = parse(&lines_of(&s));
        assert!(tf.errors.iter().any(|e| e.contains("unknown line")));
    }

    #[test]
    fn removed_fields_are_unknown_lines() {
        for field in ["test", "owner", "banned"] {
            let s = well_formed().replacen(
                "aliases: choice\n",
                &format!("aliases: choice\n{}: x\n", field),
                1,
            );
            let tf = parse(&lines_of(&s));
            assert!(
                tf.errors.iter().any(|e| e.contains("unknown line")),
                "field \"{}\" not reported as unknown line: {:?}",
                field,
                tf.errors
            );
        }
    }

    #[test]
    fn duplicate_field_is_a_shape_error() {
        let s = well_formed().replacen(
            "definition: A choice answered by the owner.\n",
            "definition: A choice answered by the owner.\ndefinition: Something else.\n",
            1,
        );
        let tf = parse(&lines_of(&s));
        assert!(tf
            .errors
            .iter()
            .any(|e| e.contains("duplicate field \"definition\"")));
    }

    #[test]
    fn missing_id_gets_the_owner_wording() {
        let s = well_formed().replacen("id: 11111111-1111-1111-1111-111111111111\n", "", 1);
        let tf = parse(&lines_of(&s));
        assert!(tf
            .errors
            .iter()
            .any(|e| e.contains("missing id; owner runs adr approve adrdom/terms.md")));
    }

    #[test]
    fn duplicate_id_is_a_shape_error() {
        let s = well_formed().replacen(
            "22222222-2222-2222-2222-222222222222",
            "11111111-1111-1111-1111-111111111111",
            1,
        );
        let tf = parse(&lines_of(&s));
        assert!(tf.errors.iter().any(|e| e.contains("duplicate id")));
    }

    #[test]
    fn duplicate_word_across_two_entries_is_an_error() {
        let s = well_formed().replacen("aliases: choice\n", "aliases: question\n", 1);
        let tf = parse(&lines_of(&s));
        assert!(tf.errors.iter().any(
            |e| e.contains("word \"question\"") && e.contains("decision") && e.contains("question")
        ));
    }

    #[test]
    fn empty_name_is_a_shape_error() {
        let s = well_formed().replacen("## decision\n", "## \n", 1);
        let tf = parse(&lines_of(&s));
        assert!(tf.errors.iter().any(|e| e.contains("empty name")));
    }

    fn sample_terms() -> Vec<Term> {
        vec![
            Term {
                name: "terms file".to_string(),
                id: Some("id-terms-file".to_string()),
                definition: String::new(),
                aliases: vec![],
                retired: false,
                start_line: 1,
                id_line: Some(2),
                terms_line: None,
                terms_line_idx: None,
            },
            Term {
                name: "terms".to_string(),
                id: Some("id-terms".to_string()),
                definition: String::new(),
                aliases: vec![],
                retired: false,
                start_line: 10,
                id_line: Some(11),
                terms_line: None,
                terms_line_idx: None,
            },
            Term {
                name: "decision".to_string(),
                id: Some("id-decision".to_string()),
                definition: String::new(),
                aliases: vec![],
                retired: false,
                start_line: 20,
                id_line: Some(21),
                terms_line: None,
                terms_line_idx: None,
            },
            Term {
                name: "verdict".to_string(),
                id: Some("id-verdict".to_string()),
                definition: String::new(),
                aliases: vec![],
                retired: true,
                start_line: 30,
                id_line: Some(31),
                terms_line: None,
                terms_line_idx: None,
            },
        ]
    }

    #[test]
    fn resolve_prefers_the_longer_phrase() {
        let terms = sample_terms();
        let pairs = resolve("read the terms file today", &terms);
        assert_eq!(
            pairs,
            vec![("terms file".to_string(), "id-terms-file".to_string())]
        );
    }

    #[test]
    fn resolve_matches_simple_plural() {
        let terms = sample_terms();
        let pairs = resolve("the decisions were many", &terms);
        assert!(
            pairs
                .iter()
                .any(|(w, id)| w == "decisions" && id == "id-decision"),
            "{:?}",
            pairs
        );

        // Not a plural of anything registered: no arbitrary stemming.
        let pairs2 = resolve("a decisional matter", &terms);
        assert!(pairs2.iter().all(|(_, id)| id != "id-decision"));

        // Exact singular still matches.
        let pairs3 = resolve("the decision was final", &terms);
        assert_eq!(
            pairs3,
            vec![("decision".to_string(), "id-decision".to_string())]
        );
    }

    #[test]
    fn resolve_plural_does_not_shadow_a_longer_phrase() {
        let terms = vec![
            Term {
                name: "cat".to_string(),
                id: Some("id-cat".to_string()),
                definition: String::new(),
                aliases: vec![],
                retired: false,
                start_line: 1,
                id_line: Some(2),
                terms_line: None,
                terms_line_idx: None,
            },
            Term {
                name: "cats meow".to_string(),
                id: Some("id-cats-meow".to_string()),
                definition: String::new(),
                aliases: vec![],
                retired: false,
                start_line: 10,
                id_line: Some(11),
                terms_line: None,
                terms_line_idx: None,
            },
        ];
        let pairs = resolve("the cats meow loudly", &terms);
        assert_eq!(
            pairs,
            vec![("cats meow".to_string(), "id-cats-meow".to_string())]
        );
    }

    #[test]
    fn resolve_matches_irregular_plural_via_alias() {
        let terms = vec![Term {
            name: "mouse".to_string(),
            id: Some("id-mouse".to_string()),
            definition: String::new(),
            aliases: vec!["mice".to_string()],
            retired: false,
            start_line: 1,
            id_line: Some(2),
            terms_line: None,
            terms_line_idx: None,
        }];
        let pairs = resolve("the mice ran", &terms);
        assert_eq!(pairs, vec![("mice".to_string(), "id-mouse".to_string())]);
        // The auto-plural of "mouse" ("mouses") is not what appears in the
        // text, and "mouse" itself doesn't appear either.
        let pairs2 = resolve("a mouse ran", &terms);
        assert_eq!(pairs2, vec![("mouse".to_string(), "id-mouse".to_string())]);
    }

    #[test]
    fn resolve_excludes_retired_terms() {
        // "verdict" is retired in `sample_terms`; a word that only a
        // retired term owns pairs nothing at all (9.3 B).
        let terms = sample_terms();
        assert!(terms.iter().find(|t| t.name == "verdict").unwrap().retired);
        let pairs = resolve("the verdict was reached", &terms);
        assert!(
            pairs.iter().all(|(_, id)| id != "id-verdict"),
            "{:?}",
            pairs
        );
        // A current term is unaffected.
        let pairs2 = resolve("a decision was made", &terms);
        assert!(pairs2
            .iter()
            .any(|(w, id)| w == "decision" && id == "id-decision"));
    }

    #[test]
    fn phrase_lookup_excludes_retired_terms() {
        let terms = sample_terms();
        let lookup = phrase_lookup(&terms);
        assert!(!lookup.values().any(|id| id == "id-verdict"));
        assert_eq!(lookup.get("decision").map(String::as_str), Some("id-decision"));
    }

    #[test]
    fn duplicate_word_errors_ignores_a_retired_entry() {
        // A retired entry and a current entry sharing the same word: no
        // conflict, since the retired one is not counted (9.3 B).
        let terms = vec![
            Term {
                name: "decision".to_string(),
                id: Some("id-old".to_string()),
                definition: String::new(),
                aliases: vec![],
                retired: true,
                start_line: 1,
                id_line: Some(2),
                terms_line: None,
                terms_line_idx: None,
            },
            Term {
                name: "decision".to_string(),
                id: Some("id-new".to_string()),
                definition: String::new(),
                aliases: vec![],
                retired: false,
                start_line: 10,
                id_line: Some(11),
                terms_line: None,
                terms_line_idx: None,
            },
        ];
        let errors = duplicate_word_errors(&terms);
        assert!(errors.is_empty(), "{:?}", errors);
    }

    // `resolve_word` (the per-call, per-entry resolver) was folded into
    // `depends::resolve_dep`, which now takes a `phrase_lookup` built once
    // per command run instead of rebuilding it on every `depends:` entry;
    // these three cases move to `phrase_lookup` directly, the map that
    // backed it and still backs `resolve_dep`.

    #[test]
    fn phrase_lookup_word_rejects_a_retired_term() {
        let terms = sample_terms();
        let lookup = phrase_lookup(&terms);
        // "verdict" is retired in `sample_terms()`.
        assert_eq!(lookup.get("verdict"), None);
    }

    #[test]
    fn phrase_lookup_word_resolves_a_current_term_exactly_and_as_a_plural() {
        let terms = sample_terms();
        let lookup = phrase_lookup(&terms);
        assert_eq!(lookup.get("terms"), Some(&"id-terms".to_string()));
        assert_eq!(
            lookup.get("terms file"),
            Some(&"id-terms-file".to_string())
        );
    }

    #[test]
    fn phrase_lookup_word_returns_none_for_an_unknown_word() {
        let terms = sample_terms();
        let lookup = phrase_lookup(&terms);
        assert_eq!(lookup.get("nosuchword"), None);
    }

    #[test]
    fn resolve_skips_entries_without_id() {
        let mut terms = sample_terms();
        terms[1].id = None;
        let pairs = resolve("terms terms", &terms);
        assert!(pairs.iter().all(|(_, id)| id != "id-terms"));
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
    fn format_and_parse_terms_line_round_trip() {
        let pairs = vec![
            (
                "decision".to_string(),
                "11111111-1111-1111-1111-111111111111".to_string(),
            ),
            (
                "question".to_string(),
                "22222222-2222-2222-2222-222222222222".to_string(),
            ),
        ];
        let line = format_terms_line(&pairs).unwrap();
        assert_eq!(
            line,
            "terms: decision=11111111-1111-1111-1111-111111111111, question=22222222-2222-2222-2222-222222222222"
        );
        let parsed = parse_terms_line(&line).unwrap();
        assert_eq!(parsed, pairs);
    }

    #[test]
    fn format_terms_line_is_none_when_empty() {
        assert!(format_terms_line(&[]).is_none());
    }

    #[test]
    fn parse_terms_line_rejects_non_terms_lines() {
        assert!(parse_terms_line("id: abc").is_none());
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

    fn term_with_definition(
        name: &str,
        id: &str,
        definition: &str,
        start_line: usize,
    ) -> Term {
        Term {
            name: name.to_string(),
            id: Some(id.to_string()),
            definition: definition.to_string(),
            aliases: vec![],
            retired: false,
            start_line,
            id_line: Some(start_line + 1),
            terms_line: None,
            terms_line_idx: None,
        }
    }

    fn tf_of(terms: Vec<Term>) -> TermsFile {
        TermsFile {
            path: PathBuf::new(),
            lines: Vec::new(),
            terms,
            errors: Vec::new(),
        }
    }

    #[test]
    fn cycles_finds_a_self_cycle() {
        let tf = tf_of(vec![term_with_definition(
            "loop",
            "id-loop",
            "A loop that refers to loop.",
            1,
        )]);
        let found = cycles(&tf);
        assert_eq!(found, vec![vec!["loop".to_string()]]);
    }

    #[test]
    fn cycles_finds_a_two_node_cycle() {
        let tf = tf_of(vec![
            term_with_definition("alpha", "id-alpha", "See beta.", 1),
            term_with_definition("beta", "id-beta", "See alpha.", 10),
        ]);
        let found = cycles(&tf);
        assert_eq!(found, vec![vec!["alpha".to_string(), "beta".to_string()]]);
    }

    #[test]
    fn cycles_finds_a_three_node_cycle() {
        let tf = tf_of(vec![
            term_with_definition("alpha", "id-alpha", "See beta.", 1),
            term_with_definition("beta", "id-beta", "See gamma.", 10),
            term_with_definition("gamma", "id-gamma", "See alpha.", 20),
        ]);
        let found = cycles(&tf);
        assert_eq!(
            found,
            vec![vec![
                "alpha".to_string(),
                "beta".to_string(),
                "gamma".to_string()
            ]]
        );
    }

    #[test]
    fn cycles_finds_none_when_the_graph_has_no_cycle() {
        let tf = tf_of(vec![
            term_with_definition("alpha", "id-alpha", "A plain first thing.", 1),
            term_with_definition("beta", "id-beta", "See alpha.", 10),
        ]);
        assert!(cycles(&tf).is_empty());
    }

    #[test]
    fn cycles_handles_a_dense_complete_graph_quickly() {
        // Every term's definition pairs every other term: one giant
        // strongly connected component. The old enumerate-every-cycle
        // approach is factorial here; the SCC approach is linear.
        let n = 40;
        let names: Vec<String> = (0..n).map(|i| format!("term{}", i)).collect();
        let terms: Vec<Term> = (0..n)
            .map(|i| {
                let definition = names
                    .iter()
                    .enumerate()
                    .filter(|&(j, _)| j != i)
                    .map(|(_, name)| format!("See {}.", name))
                    .collect::<Vec<_>>()
                    .join(" ");
                term_with_definition(&names[i], &format!("id-{}", i), &definition, i * 10 + 1)
            })
            .collect();
        let tf = tf_of(terms);

        let start = std::time::Instant::now();
        let found = cycles(&tf);
        let elapsed = start.elapsed();

        assert_eq!(found.len(), 1, "expected exactly one cycle, got {:?}", found);
        assert!(elapsed.as_secs() < 1, "cycles took too long: {:?}", elapsed);
    }
}
