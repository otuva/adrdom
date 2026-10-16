//! The index: one vector per record of the pool, kept in
//! `adrdom/cache/index.tsv`, out of git, rebuilt on any clone from the topic
//! files, the terms file and the ledger ("Does git track a file the tool can
//! get again without the owner?"). The pool is the current decisions and the
//! definitions of the terms that are not retired. The first line is
//! `# model <name> dim <n>`: an index written by another model, or with no
//! such line, is not reused. A row is `kind<TAB>id<TAB>hash<TAB>v1 v2 ... vn`;
//! the hash is the sha256 of the embedded text, so only a record whose text
//! changed is embedded again.
use crate::embed::Embedder;
use crate::ledger::Ledger;
use crate::msg;
use crate::record::{self, RecordFile};
use crate::repo;
use crate::state;
use crate::terms::Term;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub const INDEX_PATH: &str = "adrdom/cache/index.tsv";

/// How many neighbours the tool brings for a new heading ("What does the
/// tool bring as neighbours for a new heading?").
pub const NEIGHBOURS: usize = 10;

#[derive(Debug, Clone)]
pub struct Rec {
    /// `decision` or `term`.
    pub kind: &'static str,
    pub id: String,
    /// The text that is embedded: for a decision its heading and the option
    /// sentence the owner picked, for a term `name: definition`.
    pub text: String,
    pub hash: String,
}

/// One record of the pool near a query.
#[derive(Debug, Clone)]
pub struct Neighbour {
    pub kind: &'static str,
    pub id: String,
    pub score: f32,
    pub text: String,
}

fn rec(kind: &'static str, id: String, text: String) -> Rec {
    let hash = record::sha256_hex(&text);
    Rec {
        kind,
        id,
        text,
        hash,
    }
}

/// The pool, in a fixed order: every current decision, then every term that
/// is not retired.
pub fn pool(files: &[RecordFile], led: &Ledger, terms: &[Term]) -> Vec<Rec> {
    let mut out = Vec::new();
    for rf in files {
        for sec in &rf.sections {
            let Some(id) = &sec.id else { continue };
            if !state::is_current(led, id) {
                continue;
            }
            let mut t = sec.heading_text.trim().to_string();
            if let Some(s) = sec.chosen_sentence() {
                t.push('\n');
                t.push_str(s);
            }
            out.push(rec("decision", id.clone(), t));
        }
    }
    // A term with no id is not approved yet: it is not in the pool.
    for t in terms.iter().filter(|t| !t.retired) {
        let Some(id) = t.id.clone() else { continue };
        out.push(rec("term", id, format!("{}: {}", t.name, t.definition)));
    }
    out
}

type Key = (String, String, String);

/// The vectors of the records, then the vectors of the extra texts.
pub type Vectors = (Vec<Vec<f32>>, Vec<Vec<f32>>);

/// The stored vectors, or none when the file has no header or its header
/// names another model than `model`: then every record is embedded again.
/// A row whose width is not the header's, or that holds a value which is
/// not finite, is dropped.
fn read(root: &Path, model: &str) -> HashMap<Key, Vec<f32>> {
    let mut m = HashMap::new();
    let Ok(text) = fs::read_to_string(root.join(INDEX_PATH)) else {
        return m;
    };
    let mut lines = text.lines();
    let Some(h) = lines.next().and_then(|l| l.strip_prefix("# model ")) else {
        return m;
    };
    let Some((name, dim)) = h.rsplit_once(" dim ") else {
        return m;
    };
    let Ok(dim) = dim.trim().parse::<usize>() else {
        return m;
    };
    if name != model {
        return m;
    }
    for line in lines {
        let p: Vec<&str> = line.split('\t').collect();
        if p.len() != 4 {
            continue;
        }
        let v: Result<Vec<f32>, _> = p[3].split(' ').map(str::parse::<f32>).collect();
        if let Ok(v) = v {
            if v.len() == dim && v.iter().all(|x| x.is_finite()) {
                m.insert((p[0].into(), p[1].into(), p[2].into()), v);
            }
        }
    }
    m
}

/// The dot product; the vectors are L2-normalised, so it is the cosine.
pub fn dot(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// The indexes of the (at most) `n` vectors nearest to `query` among those
/// `keep` accepts, nearest first, ties by index.
pub fn rank(
    query: &[f32],
    vecs: &[Vec<f32>],
    keep: impl Fn(usize) -> bool,
    n: usize,
) -> Vec<(usize, f32)> {
    let mut scored: Vec<(f32, usize)> = (0..vecs.len())
        .filter(|&i| keep(i))
        .map(|i| (dot(query, &vecs[i]), i))
        .collect();
    scored.sort_by(|a, b| b.0.total_cmp(&a.0).then(a.1.cmp(&b.1)));
    scored.into_iter().take(n).map(|(s, i)| (i, s)).collect()
}

/// Brings the index up to date with `recs` and returns each record's vector,
/// index-aligned with `recs`. `extra` texts are embedded in the same call and
/// returned after. This is the one function that embeds texts besides the
/// records: a caller that ranks texts of its own against each other passes
/// them as `extra`.
pub fn refresh(
    root: &Path,
    emb: &dyn Embedder,
    recs: &[Rec],
    extra: &[String],
) -> Result<Vectors, String> {
    let retry = || msg::fix_then_same_command("the cache directory the error names");
    let dir = root.join(repo::CACHE_DIR);
    fs::create_dir_all(&dir).map_err(|e| msg::create_failed(repo::CACHE_DIR, e, retry()))?;
    let mut have = read(root, emb.name());
    let mut todo: Vec<usize> = Vec::new();
    let mut vecs: Vec<Option<Vec<f32>>> = Vec::new();
    for (i, r) in recs.iter().enumerate() {
        let v = have.remove(&(r.kind.to_string(), r.id.clone(), r.hash.clone()));
        if v.is_none() {
            todo.push(i);
        }
        vecs.push(v);
    }
    let mut texts: Vec<String> = todo.iter().map(|&i| recs[i].text.clone()).collect();
    let n_new = texts.len();
    texts.extend(extra.iter().cloned());
    let mut got = if texts.is_empty() {
        Vec::new()
    } else {
        emb.embed(&texts)?
    };
    let queries = got.split_off(n_new);
    for (k, v) in todo.iter().zip(got) {
        vecs[*k] = Some(v);
    }
    let vecs: Vec<Vec<f32>> = vecs.into_iter().map(|v| v.unwrap_or_default()).collect();
    let dim = vecs.first().map_or(0, |v| v.len());
    let mut out = format!("# model {} dim {}\n", emb.name(), dim);
    for (r, v) in recs.iter().zip(&vecs) {
        let nums: Vec<String> = v.iter().map(|x| x.to_string()).collect();
        out.push_str(&format!(
            "{}\t{}\t{}\t{}\n",
            r.kind,
            r.id,
            r.hash,
            nums.join(" ")
        ));
    }
    let path = root.join(INDEX_PATH);
    let old = fs::read_to_string(&path).unwrap_or_default();
    if old != out {
        fs::write(&path, out).map_err(|e| msg::write_failed(INDEX_PATH, e, retry()))?;
    }
    Ok((vecs, queries))
}

/// For each query text, the `n` records of the pool nearest to it, nearest
/// first. Loads the embedding model, brings the index up to date and embeds
/// the queries in the same call. An error when the model is missing.
pub fn nearest(
    root: &Path,
    led: &Ledger,
    queries: &[String],
    n: usize,
) -> Result<Vec<Vec<Neighbour>>, String> {
    nearest_with_vectors(root, led, queries, n).map(|(near, _)| near)
}

type NearestVectors = (Vec<Vec<Neighbour>>, Vec<Vec<f32>>);

/// `nearest`, and the vector of each query text as well, so a caller that
/// ranks the query texts against each other does not embed them again.
pub fn nearest_with_vectors(
    root: &Path,
    led: &Ledger,
    queries: &[String],
    n: usize,
) -> Result<NearestVectors, String> {
    let files = record::load_all(root);
    let terms = state::load_terms(root).map(|t| t.terms).unwrap_or_default();
    let recs = pool(&files, led, &terms);
    let emb = crate::embed::load(root)?;
    let (vecs, qv) = refresh(root, emb.as_ref(), &recs, queries)?;
    let near = qv
        .iter()
        .map(|q| {
            rank(q, &vecs, |_| true, n)
                .into_iter()
                .map(|(i, score)| Neighbour {
                    kind: recs[i].kind,
                    id: recs[i].id.clone(),
                    score,
                    text: recs[i].text.clone(),
                })
                .collect()
        })
        .collect();
    Ok((near, qv))
}

#[cfg(all(test, feature = "fake-model"))]
mod tests {
    use super::*;
    use crate::embed::Fake;
    use std::cell::Cell;

    /// Counts the texts it is asked to embed, wrapping the fake model.
    struct Counting(Cell<usize>);
    impl Embedder for Counting {
        fn name(&self) -> &'static str {
            Fake.name()
        }
        fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>, String> {
            self.0.set(self.0.get() + texts.len());
            Fake.embed(texts)
        }
    }

    fn scratch() -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!(
            "adr-index-unit-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn only_a_record_whose_text_changed_is_embedded_again() {
        let root = scratch();
        let mut recs = vec![
            rec("decision", "id-1".into(), "does check run".into()),
            rec("term", "id-2".into(), "ledger: the file".into()),
        ];
        let emb = Counting(Cell::new(0));
        refresh(&root, &emb, &recs, &[]).unwrap();
        assert_eq!(emb.0.get(), 2);
        // Nothing changed: nothing is embedded, only the query is.
        let (_, q) = refresh(&root, &emb, &recs, &["query".to_string()]).unwrap();
        assert_eq!(emb.0.get(), 3);
        assert_eq!(q.len(), 1);
        // One text changed: one record and the query are embedded again.
        recs[1] = rec("term", "id-2".into(), "ledger: the other file".into());
        refresh(&root, &emb, &recs, &["query".to_string()]).unwrap();
        assert_eq!(emb.0.get(), 5);
        // Stale rows are not kept.
        let text = fs::read_to_string(root.join(INDEX_PATH)).unwrap();
        assert!(
            text.starts_with("# model fake-bag-of-words dim 256\n"),
            "{}",
            text
        );
        assert_eq!(text.lines().count(), 3);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn an_index_of_another_model_or_without_header_is_not_reused() {
        let root = scratch();
        let recs = vec![rec("decision", "id-1".into(), "does check run".into())];
        let emb = Counting(Cell::new(0));
        refresh(&root, &emb, &recs, &[]).unwrap();
        let path = root.join(INDEX_PATH);
        let t = fs::read_to_string(&path)
            .unwrap()
            .replace("fake-bag-of-words", "other-model");
        fs::write(&path, t).unwrap();
        refresh(&root, &emb, &recs, &[]).unwrap();
        assert_eq!(emb.0.get(), 2);
        let t = fs::read_to_string(&path).unwrap();
        fs::write(&path, t.lines().skip(1).collect::<Vec<_>>().join("\n")).unwrap();
        refresh(&root, &emb, &recs, &[]).unwrap();
        assert_eq!(emb.0.get(), 3);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn rank_orders_nearest_first_and_ties_by_index() {
        let vecs = vec![vec![1.0, 0.0], vec![0.0, 1.0], vec![1.0, 0.0]];
        let r = rank(&[1.0, 0.0], &vecs, |_| true, 2);
        assert_eq!(r.iter().map(|x| x.0).collect::<Vec<_>>(), vec![0, 2]);
    }

    #[test]
    fn a_term_with_no_id_is_not_in_the_pool() {
        let term = |name: &str, id: Option<&str>| Term {
            name: name.into(),
            id: id.map(String::from),
            definition: "d".into(),
            aliases: Vec::new(),
            retired: false,
            start_line: 1,
            id_line: None,
            terms_line: None,
            terms_line_idx: None,
        };
        let terms = vec![term("new term", None), term("old", Some("id-1"))];
        let p = pool(
            &[],
            &Ledger {
                rows: Vec::new(),
                malformed: Vec::new(),
            },
            &terms,
        );
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].id, "id-1");
    }

    #[test]
    fn a_row_with_a_value_that_is_not_finite_is_dropped_and_rank_survives_it() {
        let root = scratch();
        fs::create_dir_all(root.join(repo::CACHE_DIR)).unwrap();
        fs::write(
            root.join(INDEX_PATH),
            "# model m dim 2\ndecision\ta\th\t1 0\ndecision\tb\th\tNaN 0\ndecision\tc\th\tinf 0\n",
        )
        .unwrap();
        assert_eq!(read(&root, "m").len(), 1);
        let vecs = vec![vec![f32::NAN, 0.0], vec![1.0, 0.0]];
        let r = rank(&[1.0, 0.0], &vecs, |_| true, 2);
        assert_eq!(r.len(), 2);
        fs::remove_dir_all(&root).unwrap();
    }
}
