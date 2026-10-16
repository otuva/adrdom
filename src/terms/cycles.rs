//! Circular definitions: the cycles in the graph of terms pairing terms.

use super::{definition_pairs, TermsFile};
use std::collections::{BTreeSet, HashMap};

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
    for (i, edges) in adj.iter_mut().enumerate() {
        if tf.terms[i].id.is_none() {
            continue;
        }
        for (_, id) in definition_pairs(tf, i) {
            if let Some(&j) = id_to_idx.get(id.as_str()) {
                edges.insert(j);
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
        let names = cycle_idx
            .iter()
            .map(|&i| tf.terms[i].name.clone())
            .collect();
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

#[allow(clippy::too_many_arguments)]
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
            tarjan_visit(
                v,
                adj,
                index_counter,
                indices,
                lowlink,
                on_stack,
                stack,
                sccs,
            );
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
            let pos = on_stack
                .iter()
                .position(|&x| x == v)
                .expect("v is on the stack");
            *found = Some(on_stack[pos..].to_vec());
            break;
        } else if !visited.contains(&v) {
            find_cycle_dfs(v, members, adj, visited, on_stack, on_stack_set, found);
        }
    }

    on_stack.pop();
    on_stack_set.remove(&u);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terms::tests::{lines_of, well_formed};
    use crate::terms::{minted_copy, parse, Term};
    use std::path::PathBuf;

    fn term_with_definition(name: &str, id: &str, definition: &str, start_line: usize) -> Term {
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
        assert!(
            cycles(&tf).is_empty(),
            "id-less file should show no cycle directly"
        );

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

        assert_eq!(
            found.len(),
            1,
            "expected exactly one cycle, got {:?}",
            found
        );
        assert!(elapsed.as_secs() < 1, "cycles took too long: {:?}", elapsed);
    }
}
