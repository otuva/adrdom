//! The rule on a planner's commit: it holds files under adrdom only.
use super::Failure;
use crate::rules::Class;
use crate::session::{self, Role};
use crate::{msg, repo};
use std::path::Path;

/// How many paths the finding names before "and N more".
const NAMED: usize = 3;

/// The failure for a commit in a planner's session that stages a path
/// outside adrdom (a deletion or either side of a rename counts), or whose
/// staged paths cannot be read. Nothing in any other session.
pub(super) fn check_commit(root: &Path) -> Vec<Failure> {
    match session::role_checked(root) {
        Ok(Role::Planner) => {}
        Ok(Role::Plain) => return Vec::new(),
        Err(e) => {
            return vec![Failure::said(
                Class::Other,
                msg::planner_roles_unreadable(repo::ROLES_PATH, &e),
            )]
        }
    }
    let prefix = format!("{}/", repo::ADRDOM_DIR);
    let outside: Vec<String> = match repo::staged_paths(root, true) {
        Some(paths) => paths
            .into_iter()
            .filter(|p| !p.starts_with(&prefix))
            .collect(),
        None => {
            return vec![Failure::said(
                Class::Other,
                msg::planner_staged_unreadable(),
            )]
        }
    };
    if outside.is_empty() {
        return Vec::new();
    }
    vec![Failure::said(
        Class::Other,
        msg::planner_commit_outside(&outside, NAMED),
    )]
}
