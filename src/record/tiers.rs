//! The tier of every section, derived from its `depends:` entries, and the cap
//! on a tier.

use super::*;

// --- tiers ---

/// The cap on how many live questions one derived tier may hold. A different
/// value is a new question and a new release; no owner command sets it.
pub const TIER_CAP: usize = 4;

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

/// `(tier, count)` for every tier that has at least one section, sorted by
/// tier.
pub fn tier_sizes(tiers: &[usize]) -> Vec<(usize, usize)> {
    let mut counts: std::collections::BTreeMap<usize, usize> = std::collections::BTreeMap::new();
    for &t in tiers {
        *counts.entry(t).or_insert(0) += 1;
    }
    counts.into_iter().collect()
}

/// The first (lowest-numbered) tier holding more than `TIER_CAP` sections,
/// if any (per "Does the tool cap the questions in a tier?").
pub fn over_cap(tiers: &[usize]) -> Option<(usize, usize)> {
    tier_sizes(tiers)
        .into_iter()
        .find(|&(_, count)| count > TIER_CAP)
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
    fn over_cap_finds_the_first_tier_past_the_cap() {
        assert_eq!(TIER_CAP, 4);
        let tiers = vec![1, 1, 1, 1, 1];
        assert_eq!(over_cap(&tiers), Some((1, 5)));
        assert_eq!(tier_sizes(&tiers), vec![(1, 5)]);

        let ok = vec![1, 1, 1, 1];
        assert_eq!(over_cap(&ok), None);

        let mixed = vec![1, 1, 2, 2, 2, 2, 2];
        assert_eq!(over_cap(&mixed), Some((2, 5)));
        assert_eq!(tier_sizes(&mixed), vec![(1, 2), (2, 5)]);
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
