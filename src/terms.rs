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

/// `"key:value"` or `"key: value"`, trimmed: unlike `record::kv`, the space
/// after the colon is optional, since a real `adrdom/terms.md` field with
/// an empty value is written with no trailing space at all (e.g. an
/// aliases-less term's bare `aliases:` line) -- `record::kv` would treat
/// that as not matching the key at all.
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
/// may take a retired term's word (per "Does a retired term keep its words?"); two current entries sharing a
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

/// Sentences in `text` (fcc1c1ec-d8e8-428e-9731-9817cdeb8e76): a `.`, `?`
/// or `!` ends a sentence only when it is followed by the end of the text,
/// or by whitespace and then an uppercase letter, a digit, or an opening
/// quote, backtick, or bracket -- so an abbreviation like "e.g." or "i.e."
/// mid-sentence, whose next word starts lowercase, is not counted as a
/// sentence end.
pub fn sentence_count(text: &str) -> usize {
    let chars: Vec<char> = text.chars().collect();
    let mut count = 0;
    for (i, &c) in chars.iter().enumerate() {
        if c != '.' && c != '?' && c != '!' {
            continue;
        }
        if i + 1 == chars.len() {
            count += 1;
            continue;
        }
        if !chars[i + 1].is_whitespace() {
            continue;
        }
        let next = chars[(i + 1)..].iter().find(|c| !c.is_whitespace());
        match next {
            None => count += 1,
            Some(&n) => {
                if n.is_uppercase()
                    || n.is_ascii_digit()
                    || matches!(n, '"' | '\'' | '`' | '(' | '[' | '{')
                {
                    count += 1;
                }
            }
        }
    }
    count
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

fn is_boundary_ok(chars: &[char], start: usize, len: usize) -> bool {
    let before_ok = start == 0 || !crate::text::is_word_char(chars[start - 1]);
    let after_ok = start + len == chars.len() || !crate::text::is_word_char(chars[start + len]);
    before_ok && after_ok
}

/// Every name and alias in `terms`, lowercased, mapped to the id of the term
/// that owns it. A word pairs to a term only when it equals the term's name
/// or one of its aliases exactly -- no implicit plural or other stemming
/// (per "Does a term word keep its plain sense in the records?"): every form
/// that should pair is written as an explicit alias. Terms without an id,
/// and retired terms, are skipped (per "Does a retired term keep its
/// words?"): a lookup never lands on a retired term, so a new pairing sees
/// current terms only.
///
/// A single lookup against this map answers whether a whole word/phrase
/// still resolves to a given id exactly as `resolve(word, terms)` would
/// (a stored pairing is always the whole matched text, so only a
/// full-string candidate can produce it); built once and reused across
/// many words instead of rebuilding `resolve`'s phrase list per word.
pub fn phrase_lookup(terms: &[Term]) -> HashMap<String, String> {
    let mut map: HashMap<String, String> = HashMap::new();
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
            map.entry(nl).or_insert_with(|| id.clone());
        }
    }
    map
}

/// Whole-word/phrase, case-insensitive, longest-phrase-first, non-overlapping
/// resolution of every term name/alias in `text`. Retired terms are never
/// matched (per "Does a retired term keep its words?"): every new pairing sees current terms only, so a word
/// that only a retired term owns resolves to nothing here (a current term
/// is free to take that same name or alias). Terms without an id are
/// skipped. Returned pairs are unique and sorted by matched word.
///
/// A word matches a name or alias only when it equals that name/alias
/// exactly (per "Does a term word keep its plain sense in the records?"):
/// there is no implicit plural or other stemming, so a plural or other
/// variant form pairs only when it is itself written as an alias.
pub fn resolve(text: &str, terms: &[Term]) -> Vec<(String, String)> {
    let mut phrases: Vec<(Vec<char>, String)> = Vec::new();
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
            phrases.push((nl.chars().collect(), id.clone()));
        }
    }
    // Longest phrase first; ties broken lexicographically for determinism.
    phrases.sort_by(|a, b| {
        b.0.len().cmp(&a.0.len()).then_with(|| {
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
        for (phrase, id) in &phrases {
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

/// A single uuid on a `terms:` line that no longer names an existing,
/// non-retired term.
pub enum ProblemKind {
    /// The uuid names no term in the file at all.
    Gone,
    /// The uuid names a term, but it is retired; carries the term's name.
    Retired(String),
}

/// One `Problem` per uuid `validate_pairs` finds wrong on a line.
pub struct Problem {
    pub uuid: String,
    /// The word the line names the term by.
    pub word: String,
    pub kind: ProblemKind,
}

/// The line does not parse as a `terms:` line at all (per
/// `parse_terms_line`: missing the `terms: ` prefix, or nothing after it).
#[derive(Debug)]
pub struct Malformed;

/// Validates every uuid on one `terms:` line (`line`, exactly as it stands
/// in the file) against `tf`: `Err(Malformed)` when the line itself doesn't
/// parse, otherwise `Ok` with one `Problem`, in the line's own order, for
/// every uuid that no longer names an existing, non-retired term. Shared by
/// every reader that judges a single terms line this way: `actions::do_answer`
/// (refuses on the first problem it finds), and `check` (reports every one,
/// for both a section's terms line and a term's own).
pub fn validate_pairs(line: &str, tf: &TermsFile) -> Result<Vec<Problem>, Malformed> {
    let pairs = parse_terms_line(line).ok_or(Malformed)?;
    let mut problems = Vec::new();
    for (word, uuid) in pairs {
        match tf.terms.iter().find(|t| t.id.as_deref() == Some(uuid.as_str())) {
            None => problems.push(Problem {
                uuid,
                word,
                kind: ProblemKind::Gone,
            }),
            Some(t) if t.retired => problems.push(Problem {
                uuid,
                word,
                kind: ProblemKind::Retired(t.name.clone()),
            }),
            _ => {}
        }
    }
    Ok(problems)
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

    // adr: 7c306235-6243-4d79-b9aa-08ed19662779
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

    // adr: c6da863d-5085-402f-815a-9bb832b4df7e
    // A term entry carries no `banned` field: the ban list is the only
    // place a term's banned words are kept.
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

    // adr: fcc1c1ec-d8e8-428e-9731-9817cdeb8e76
    #[test]
    fn sentence_count_counts_terminators_followed_by_whitespace_or_end() {
        assert_eq!(sentence_count("What the thing is."), 1);
        assert_eq!(sentence_count("What the thing is. Never what it does."), 2);
        assert_eq!(
            sentence_count("What the thing is. Never what it does. Or how it is used."),
            3
        );
        // A `.` not followed by whitespace or end (e.g. inside a word) is
        // not a terminator.
        assert_eq!(sentence_count("Version 1.5 of the thing."), 1);
    }

    // adr: fcc1c1ec-d8e8-428e-9731-9817cdeb8e76
    //
    // A `.` inside an abbreviation like "e.g." or "i.e." is followed by
    // whitespace, but the word after that whitespace starts lowercase, so
    // it is not a sentence end; only the terminator at the true end of the
    // text counts.
    #[test]
    fn an_abbreviation_dot_mid_sentence_does_not_end_a_sentence() {
        assert_eq!(sentence_count("Use e.g. the shared helper for this."), 1);
        assert_eq!(sentence_count("Ship i.e. one path only."), 1);
        assert_eq!(sentence_count("Ship it, etc. and stop there."), 1);
        assert_eq!(
            sentence_count("One sentence here. Another sentence here."),
            2
        );
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

    // adr: 521e6211-344a-4b56-a1aa-07be956b3b29
    //
    // "a term word always means the term in the records; its plain sense
    // leaves them, forms that mean the term are aliases, and plain use of
    // the same form is caught at review": there is no implicit "s"/"es"
    // plural match any more, so a plural that is not listed as an alias is
    // a plain word, and the exact same form pairs once it is added as an
    // alias.
    #[test]
    fn a_plural_form_pairs_only_when_it_is_an_explicit_alias() {
        let terms = sample_terms();

        // "decisions" is not an alias of "decision" here: a plain word,
        // caught at review rather than paired.
        let pairs = resolve("the decisions were many", &terms);
        assert!(pairs.iter().all(|(_, id)| id != "id-decision"), "{:?}", pairs);

        // Not a plural of anything registered either: no arbitrary stemming.
        let pairs2 = resolve("a decisional matter", &terms);
        assert!(pairs2.iter().all(|(_, id)| id != "id-decision"));

        // Exact singular still matches.
        let pairs3 = resolve("the decision was final", &terms);
        assert_eq!(
            pairs3,
            vec![("decision".to_string(), "id-decision".to_string())]
        );

        // The same plural form pairs once it is written as an explicit
        // alias.
        let mut terms_with_alias = terms.clone();
        terms_with_alias
            .iter_mut()
            .find(|t| t.name == "decision")
            .unwrap()
            .aliases
            .push("decisions".to_string());
        let pairs4 = resolve("the decisions were many", &terms_with_alias);
        assert_eq!(
            pairs4,
            vec![("decisions".to_string(), "id-decision".to_string())]
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

    // adr: 521e6211-344a-4b56-a1aa-07be956b3b29
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
        // No automatic plural is formed from "mouse" at all; "mice" pairs
        // only because it is listed as an explicit alias.
        let pairs2 = resolve("a mouse ran", &terms);
        assert_eq!(pairs2, vec![("mouse".to_string(), "id-mouse".to_string())]);
    }

    #[test]
    fn resolve_excludes_retired_terms() {
        // "verdict" is retired in `sample_terms`; a word that only a
        // retired term owns pairs nothing at all (per "Does a retired term keep its words?").
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
        // conflict, since the retired one is not counted (per "Does a retired term keep its words?").
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
    fn phrase_lookup_word_resolves_a_current_term_by_name_or_phrase() {
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

    // adr: 5892981c-f213-404f-b93f-551e4d081ebf -- the terms list, like
    // depends and supersedes, is one line of comma-separated entries: two
    // pairs round-trip through one `terms:` line, not two.
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
    fn validate_pairs_reports_malformed_on_a_line_that_does_not_parse() {
        let tf = parse(&lines_of(&well_formed()));
        assert!(validate_pairs("id: abc", &tf).is_err());
        assert!(validate_pairs("terms: ", &tf).is_err());
    }

    #[test]
    fn validate_pairs_reports_a_gone_uuid() {
        let tf = parse(&lines_of(&well_formed()));
        let problems = validate_pairs("terms: decision=99999999-9999-9999-9999-999999999999", &tf).unwrap();
        assert_eq!(problems.len(), 1);
        assert!(matches!(problems[0].kind, ProblemKind::Gone));
        assert_eq!(problems[0].uuid, "99999999-9999-9999-9999-999999999999");
    }

    #[test]
    fn validate_pairs_reports_a_retired_term() {
        let mut s = well_formed();
        s = s.replacen("aliases: choice\n", "aliases: choice\nretired: yes\n", 1);
        let tf = parse(&lines_of(&s));
        let problems =
            validate_pairs("terms: decision=11111111-1111-1111-1111-111111111111", &tf).unwrap();
        assert_eq!(problems.len(), 1);
        match &problems[0].kind {
            ProblemKind::Retired(name) => assert_eq!(name, "decision"),
            ProblemKind::Gone => panic!("expected Retired"),
        }
    }

    #[test]
    fn validate_pairs_is_empty_when_every_pair_still_resolves() {
        let tf = parse(&lines_of(&well_formed()));
        let problems =
            validate_pairs("terms: decision=11111111-1111-1111-1111-111111111111", &tf).unwrap();
        assert!(problems.is_empty());
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
    fn cycles_misses_a_cycle_in_an_id_less_file() {
        // Documents the gap `minted_copy` exists to close: `cycles` only
        // follows edges between terms that have an id, so a term with none
        // pairs against nothing at all, even a self-reference.
        let mut tf = tf_of(vec![term_with_definition(
            "loop",
            "id-loop",
            "A loop that refers to loop.",
            1,
        )]);
        tf.terms[0].id = None;
        assert!(cycles(&tf).is_empty());
    }

    #[test]
    fn minted_copy_makes_cycles_visible_in_an_id_less_file() {
        let lines = lines_of(
            "# Terms\n\n\
## alpha\n\
definition: See beta.\n\
aliases: \n\
\n\
## beta\n\
definition: See alpha.\n\
aliases: \n",
        );
        let tf = parse(&lines);
        assert!(tf.terms.iter().all(|t| t.id.is_none()));
        assert!(cycles(&tf).is_empty(), "id-less file should show no cycle directly");

        let minted = minted_copy(&tf);
        assert!(minted.terms.iter().all(|t| t.id.is_some()));
        let found = cycles(&minted);
        assert_eq!(found, vec![vec!["alpha".to_string(), "beta".to_string()]]);
    }

    #[test]
    fn minted_copy_keeps_existing_ids() {
        // `mint_ids` (which `minted_copy` calls) always reparses from
        // `tf.lines`, so this needs a real parsed file -- not `tf_of`,
        // whose fabricated `TermsFile` carries terms with no matching
        // lines at all.
        let tf = parse(&lines_of(&well_formed()));
        let before: Vec<Option<String>> = tf.terms.iter().map(|t| t.id.clone()).collect();
        let minted = minted_copy(&tf);
        let after: Vec<Option<String>> = minted.terms.iter().map(|t| t.id.clone()).collect();
        assert_eq!(before, after);
        assert!(after.iter().all(Option::is_some));
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
