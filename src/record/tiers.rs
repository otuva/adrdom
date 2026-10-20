//! The tier of every section, derived from its `depends:` entries, and the
//! in-file questions each section stands on.

use super::*;

// --- tiers ---

/// The cap on how many live questions may stand on one question, or on terms
/// alone (per "What bounds the questions standing on one question?"). A
/// different value is a new question and a new release; no owner command sets
/// it.
pub const STAND_CAP: usize = 4;

/// The most questions one depends line may name (per "What bounds the
/// questions a depends line names?"): terms do not count.
pub const DEPENDS_LINE_CAP: usize = 2;

/// Assigns every section a 1-based tier: a section with no heading
/// `depends:` entry is tier 1 (a root); otherwise its tier is one more than
/// the highest tier among the sections its heading entries name. Term and
/// decision entries never affect tier (their resolution is checked by the
/// commands, not here).
///
/// Self-dep and dangling entries are found and reported (in file order)
/// before any tier is computed; a cycle is found by the recursive descent
/// below and reports the headings on the cycle in traversal order.
pub fn derive_tiers(rf: &RecordFile) -> Result<Vec<usize>, DeriveError> {
    let n = rf.sections.len();
    let deps = heading_deps(rf)?;

    let mut tier: Vec<Option<usize>> = vec![None; n];
    let mut state: Vec<u8> = vec![0; n];
    let mut path: Vec<usize> = Vec::new();
    for i in 0..n {
        if state[i] == 0 {
            derive_visit(i, rf, &deps, &mut tier, &mut state, &mut path)?;
        }
    }
    Ok(tier.into_iter().map(|t| t.unwrap_or(1)).collect())
}

/// The in-file questions each section names on its depends line, as indexes
/// into `rf.sections`, in line order: what the tiers and the stand cap are
/// read from. Self-dep and dangling entries are errors.
fn heading_deps(rf: &RecordFile) -> Result<Vec<Vec<usize>>, DeriveError> {
    let n = rf.sections.len();

    let mut id_idx: HashMap<&str, usize> = HashMap::new();
    for (i, sec) in rf.sections.iter().enumerate() {
        if let Some(id) = &sec.id {
            id_idx.entry(id.as_str()).or_insert(i);
        }
    }
    let mut heading_idx: HashMap<&str, usize> = HashMap::new();
    for (i, sec) in rf.sections.iter().enumerate() {
        heading_idx.entry(sec.heading_text.trim()).or_insert(i);
    }

    let mut deps: Vec<Vec<usize>> = vec![Vec::new(); n];
    for (i, sec) in rf.sections.iter().enumerate() {
        for d in &sec.depends {
            match d {
                // An `Id` entry is a heading dep only when it matches a
                // heading id in this same file (per "What does a reference in a topic file
                // carry?"); otherwise it is a term or decision reference, resolved elsewhere, and
                // never affects tier -- never a Dangling error either, since it may legitimately
                // resolve outside this file.
                Dep::Id { id, .. } => {
                    if let Some(&j) = id_idx.get(id.as_str()) {
                        if j == i {
                            return Err(DeriveError::SelfDep(sec.heading_text.clone()));
                        }
                        deps[i].push(j);
                    }
                }
                // A `Text` entry is a heading dep only when heading-shaped
                // (ends in `?`); this is the pre-approval form, resolved
                // against headings in this file in memory so `show` can
                // still derive tiers before `approve` has minted anything.
                Dep::Text { text, .. } => {
                    let trimmed = text.trim();
                    if !trimmed.ends_with('?') {
                        continue;
                    }
                    if trimmed == sec.heading_text.trim() {
                        return Err(DeriveError::SelfDep(sec.heading_text.clone()));
                    }
                    match heading_idx.get(trimmed) {
                        Some(&j) => deps[i].push(j),
                        None => {
                            return Err(DeriveError::Dangling {
                                section: sec.heading_text.clone(),
                                entry: text.clone(),
                            })
                        }
                    }
                }
            }
        }
    }
    Ok(deps)
}

fn derive_visit(
    i: usize,
    rf: &RecordFile,
    deps: &[Vec<usize>],
    tier: &mut [Option<usize>],
    state: &mut [u8],
    path: &mut Vec<usize>,
) -> Result<usize, DeriveError> {
    if let Some(t) = tier[i] {
        return Ok(t);
    }
    if state[i] == 1 {
        let pos = path.iter().position(|&x| x == i).expect("i is on the path");
        let cycle_headings: Vec<String> = path[pos..]
            .iter()
            .map(|&x| rf.sections[x].heading_text.clone())
            .collect();
        return Err(DeriveError::Cycle(cycle_headings));
    }
    state[i] = 1;
    path.push(i);
    let mut max_dep_tier = 0usize;
    for &j in &deps[i] {
        let t = derive_visit(j, rf, deps, tier, state, path)?;
        if t > max_dep_tier {
            max_dep_tier = t;
        }
    }
    path.pop();
    state[i] = 2;
    let t = if deps[i].is_empty() {
        1
    } else {
        max_dep_tier + 1
    };
    tier[i] = Some(t);
    Ok(t)
}

/// `heading_deps` with repeats dropped: the questions of its file each
/// section stands on. A section with an empty list stands on terms alone.
/// Empty lists throughout when the depends lines do not resolve (a derive
/// error, reported on its own).
pub fn in_file_parents(rf: &RecordFile) -> Vec<Vec<usize>> {
    match heading_deps(rf) {
        Ok(deps) => deps
            .into_iter()
            .map(|ds| {
                let mut out: Vec<usize> = Vec::new();
                for j in ds {
                    if !out.contains(&j) {
                        out.push(j);
                    }
                }
                out
            })
            .collect(),
        Err(_) => vec![Vec::new(); rf.sections.len()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::tests::rf_from;

    #[test]
    fn derive_tiers_roots_and_chain() {
        let rf = rf_from(
            "# Topic\n\n\
             ### A?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
             ### B?\ndepends: A?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
             ### C?\ndepends: B?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        );
        assert_eq!(rf.tiers, Some(vec![1, 2, 3]));
        assert_eq!(rf.tier_of(0), Some(1));
        assert_eq!(rf.tier_of(2), Some(3));
        assert!(rf.derive_error.is_none());
    }

    #[test]
    fn derive_tiers_dangling_entry_errors() {
        let rf = rf_from(
            "# Topic\n\n### A?\ndepends: Missing?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        );
        assert!(rf.tiers.is_none());
        match &rf.derive_error {
            Some(DeriveError::Dangling { section, entry }) => {
                assert_eq!(section, "A?");
                assert_eq!(entry, "Missing?");
            }
            other => panic!("expected Dangling, got {:?}", other),
        }
    }

    #[test]
    fn derive_tiers_cycle_errors() {
        let rf = rf_from(
            "# Topic\n\n\
             ### A?\ndepends: B?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
             ### B?\ndepends: A?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        );
        match &rf.derive_error {
            Some(DeriveError::Cycle(names)) => {
                assert!(names.contains(&"A?".to_string()), "{:?}", names);
                assert!(names.contains(&"B?".to_string()), "{:?}", names);
            }
            other => panic!("expected Cycle, got {:?}", other),
        }
    }

    #[test]
    fn derive_tiers_self_dep_errors() {
        let rf = rf_from(
            "# Topic\n\n### A?\ndepends: A?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        );
        match &rf.derive_error {
            Some(DeriveError::SelfDep(h)) => assert_eq!(h, "A?"),
            other => panic!("expected SelfDep, got {:?}", other),
        }
    }

    #[test]
    fn in_file_parents_resolves_ids_and_headings() {
        let body = "Problem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n";
        let i1 = "11111111-1111-1111-1111-111111111111";
        let i2 = "22222222-2222-2222-2222-222222222222";
        let text = format!(
            "# Topic\n\n\
             ### A?\n\nid: {i1}\n\n{body}\
             ### B?\n\nid: {i2}\n\ndepends: {i1}, A?, {i1}\n\n{body}\
             ### C?\n\ndepends: B?, A?, widget\n\n{body}"
        );
        let rf = rf_from(&text);
        assert!(rf.derive_error.is_none(), "{:?}", rf.derive_error);
        // B: id entry resolved, the heading text entry and the repeated id
        // name the same section and are skipped.
        // C: heading entries in line order, the term entry skipped.
        assert_eq!(in_file_parents(&rf), vec![vec![], vec![0], vec![1, 0]]);
    }

    #[test]
    fn in_file_parents_skips_self_and_unresolved_entries() {
        let rf = rf_from(
            "# Topic\n\n### A?\n\nid: 11111111-1111-1111-1111-111111111111\n\ndepends: 33333333-3333-3333-3333-333333333333, widget\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        );
        assert_eq!(in_file_parents(&rf), vec![Vec::<usize>::new()]);
    }

    #[test]
    fn in_file_parents_is_empty_when_a_heading_entry_dangles() {
        let body = "Problem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n";
        let rf = rf_from(&format!(
            "# Topic\n\n### A?\n\n{body}### B?\n\ndepends: A?, Nowhere?\n\n{body}"
        ));
        assert!(rf.derive_error.is_some());
        assert_eq!(in_file_parents(&rf), vec![Vec::<usize>::new(), Vec::new()]);
    }

    #[test]
    fn in_file_parents_skips_a_self_entry() {
        // A self entry is a derive error, but the sections still parse.
        let rf = rf_from(
            "# Topic\n\n### A?\n\nid: 11111111-1111-1111-1111-111111111111\n\ndepends: 11111111-1111-1111-1111-111111111111, A?\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        );
        assert!(rf.derive_error.is_some());
        assert_eq!(in_file_parents(&rf), vec![Vec::<usize>::new()]);
    }

    #[test]
    fn derive_tiers_resolves_a_heading_dep_written_as_an_id() {
        // Post-approval form: the depends line already names the target's
        // id rather than its heading text; tiers must still derive from it,
        // and a self-dep by id must still be caught.
        let rf = rf_from(
            "# Topic\n\n\
             ### A?\n\nid: 11111111-1111-1111-1111-111111111111\n\ndepends: widget\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n\n\
             ### B?\n\nid: 22222222-2222-2222-2222-222222222222\n\ndepends: 11111111-1111-1111-1111-111111111111\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        );
        assert_eq!(rf.tiers, Some(vec![1, 2]), "{:?}", rf.derive_error);
        assert!(rf.derive_error.is_none());

        let selfdep = rf_from(
            "# Topic\n\n### A?\n\nid: 11111111-1111-1111-1111-111111111111\n\ndepends: 11111111-1111-1111-1111-111111111111\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        );
        match &selfdep.derive_error {
            Some(DeriveError::SelfDep(h)) => assert_eq!(h, "A?"),
            other => panic!("expected SelfDep, got {:?}", other),
        }
    }

    #[test]
    fn derive_tiers_treats_an_unmatched_id_as_a_non_heading_reference() {
        // An `Id` entry that names no heading in this file (a term or a
        // decision elsewhere) must never be a Dangling error: only `Text`
        // heading-shaped entries can dangle.
        let rf = rf_from(
            "# Topic\n\n### A?\n\ndepends: 33333333-3333-3333-3333-333333333333\n\nProblem: p.\n\nStandard: s.\n\n- A) x.\n  Why: w.\n  Costs: c.\n- B) y.\n  Why: w.\n  Costs: c.\n",
        );
        assert!(rf.derive_error.is_none(), "{:?}", rf.derive_error);
        assert_eq!(rf.tiers, Some(vec![1]));
    }
}
