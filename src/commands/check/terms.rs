use super::Failure;
use crate::actions;
use crate::ledger::Ledger;
use crate::record::{self, RecordFile};
use crate::repo;
use crate::state::{self, rel_path};
use crate::terms;
use std::collections::HashSet;
use std::path::Path;

/// `terms`: shape/approval failures on `adrdom/terms.md` (read from the
/// index when `staged` and the file is staged there, otherwise the working
/// tree). Missing or unapproved is a failure once any topic file exists,
/// and a note otherwise. Every asked/answered section's terms line, and
/// every non-retired term's own terms line, is checked for an existing,
/// non-retired uuid on each pair -- never by re-resolving the underlying
/// text. Circular definitions fail. Returns the failures, whether the
/// terms file itself is still awaiting approval (only meaningful when no
/// topic file exists yet), and any notes ("paired by nothing").
pub(super) fn check_terms(
    root: &Path,
    records: &[RecordFile],
    led: &Ledger,
    staged: bool,
) -> (Vec<Failure>, bool, Vec<String>) {
    let mut out = Vec::new();
    let mut notes = Vec::new();
    let has_records = !repo::list_decision_files(root).is_empty();

    let Some(content) = read_terms_content(root, staged) else {
        if has_records {
            out.push(Failure {
                check: "terms".to_string(),
                id: None,
                message: "adrdom/terms.md missing".to_string(),
            });
        }
        return (out, false, notes);
    };

    let lines = record::split_lines(&content);
    let tf = terms::parse(&lines);

    // A missing id is not a shape error the agent can fix: only approve
    // mints ids. It is reported once, as part of the approval state below,
    // and only fails when a topic file exists -- never as one of
    // `terms_refusals`' shape messages, which run on a minted copy (see
    // below) and so never see it.
    let missing_ids = tf.errors.iter().any(|e| e.contains("missing id"));

    let file_hash = terms::file_hash(&tf);
    let approved = led
        .latest_for_path("terms", "adrdom/terms.md")
        .map(|r| r.hash == file_hash)
        .unwrap_or(false);
    let mut awaiting = false;
    if !approved || missing_ids {
        if has_records {
            out.push(Failure {
                check: "terms".to_string(),
                id: None,
                message: "adrdom/terms.md not approved; owner runs: adr approve adrdom/terms.md"
                    .to_string(),
            });
        } else {
            awaiting = true;
        }
    }

    // Shared with approve (E1.1): every shape, circular-definition,
    // dependent-pairing, and banned-word refusal `adrdom/terms.md` earns,
    // computed on a copy with every id minted in memory so a cycle or a
    // dependent's pairing in a terms file that has never been approved --
    // and so has no ids on disk -- is caught the same way `approve` catches
    // it (`terms::cycles` and `terms::phrase_lookup` only see a term that
    // has an id).
    let minted = terms::minted_copy(&tf);
    let (refusals, _pairs) = actions::terms_refusals(root, &minted, led);
    for r in refusals {
        out.push(Failure {
            check: "terms".to_string(),
            id: None,
            message: r,
        });
    }

    // Every asked/answered section: must have a terms line; each uuid on
    // it must name a term that exists and is not retired. No
    // re-resolution of the section text against today's terms.
    for rf in records {
        let rel = rel_path(root, &rf.path);
        for sec in &rf.sections {
            let Some(id) = &sec.id else { continue };
            let st = led.latest_state(id).map(|r| r.event.as_str());
            if !matches!(st, Some("asked") | Some("answered")) {
                continue;
            }
            let Some(tl) = sec.terms_line else {
                out.push(Failure {
                    check: "terms".to_string(),
                    id: Some(id.clone()),
                    message: format!("{}: {}: no terms line", rel, sec.heading_text),
                });
                continue;
            };
            let pairs = terms::parse_terms_line(rf.lines[tl].trim_end()).unwrap_or_default();
            for (_, uuid) in &pairs {
                match tf.terms.iter().find(|t| t.id.as_deref() == Some(uuid.as_str())) {
                    None => out.push(Failure {
                        check: "terms".to_string(),
                        id: Some(id.clone()),
                        message: format!(
                            "{}: {}: term {} not found",
                            rel, sec.heading_text, uuid
                        ),
                    }),
                    Some(t) if t.retired => out.push(Failure {
                        check: "terms".to_string(),
                        id: Some(id.clone()),
                        message: format!(
                            "{}: {}: bound term \"{}\" is retired",
                            rel, sec.heading_text, t.name
                        ),
                    }),
                    Some(_) => {}
                }
            }
        }
    }

    // Every non-retired term's own terms line: the same two checks.
    for t in &tf.terms {
        if t.retired {
            continue;
        }
        let Some(tl) = &t.terms_line else { continue };
        let pairs = terms::parse_terms_line(tl).unwrap_or_default();
        for (_, uuid) in &pairs {
            match tf.terms.iter().find(|o| o.id.as_deref() == Some(uuid.as_str())) {
                None => out.push(Failure {
                    check: "terms".to_string(),
                    id: None,
                    message: format!("adrdom/terms.md: \"{}\": term {} not found", t.name, uuid),
                }),
                Some(o) if o.retired => out.push(Failure {
                    check: "terms".to_string(),
                    id: None,
                    message: format!(
                        "adrdom/terms.md: \"{}\": bound term \"{}\" is retired",
                        t.name, o.name
                    ),
                }),
                Some(_) => {}
            }
        }
    }

    // Note (never a failure): every non-retired term that no current
    // decision's terms line, and no non-retired term's own terms line,
    // pairs against.
    let mut used: HashSet<String> = HashSet::new();
    for rf in records {
        for sec in &rf.sections {
            let Some(id) = &sec.id else { continue };
            if !state::is_current(led, id) {
                continue;
            }
            let Some(tl) = sec.terms_line else { continue };
            if let Some(pairs) = terms::parse_terms_line(rf.lines[tl].trim_end()) {
                for (_, uuid) in pairs {
                    used.insert(uuid);
                }
            }
        }
    }
    for t in &tf.terms {
        if t.retired {
            continue;
        }
        let Some(tl) = &t.terms_line else { continue };
        if let Some(pairs) = terms::parse_terms_line(tl) {
            for (_, uuid) in pairs {
                used.insert(uuid);
            }
        }
    }
    let mut unused: Vec<String> = tf
        .terms
        .iter()
        .filter(|t| !t.retired)
        .filter(|t| t.id.as_ref().map(|id| !used.contains(id)).unwrap_or(true))
        .map(|t| t.name.clone())
        .collect();
    unused.sort();
    if !unused.is_empty() {
        notes.push(format!(
            "terms: {} terms paired by nothing: {}",
            unused.len(),
            unused.join(", ")
        ));
    }

    (out, awaiting, notes)
}

/// `adrdom/terms.md`'s content for `check`: when `staged`, the index copy
/// whenever the file is tracked in the index at all (whether or not this
/// commit changes it), so a dirty unstaged working-tree edit never leaks
/// into a staged check; the working tree is used only when the path isn't
/// in the index. When not `staged`, always the working tree. `None` when
/// the file exists in neither place.
fn read_terms_content(root: &Path, staged: bool) -> Option<String> {
    if staged {
        let tracked = repo::git(
            root,
            &["ls-files", "--error-unmatch", "adrdom/terms.md"],
        )
        .is_some();
        if tracked {
            // In the index: the index copy is the answer either way -- a
            // read failure here is not a reason to fall back to the
            // (untrusted, possibly dirty) working tree.
            return repo::git(root, &["show", ":adrdom/terms.md"]);
        }
    }
    std::fs::read_to_string(root.join("adrdom/terms.md")).ok()
}
