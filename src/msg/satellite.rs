//! The messages of a satellite and its primary: where the record is, what
//! install writes on both sides, and what check holds the copies to.

use super::*;

// --- satellite.rs -----------------------------------------------------------

/// A command that writes the record, run in a satellite: the record is at
/// `primary` (per "Does a command that writes the record run in a
/// satellite?"), so the command is run again there. `actor` is who runs
/// `cmd`: the owner for the owner's commands, the agent for the others.
pub fn record_elsewhere(cmd: &str, args: &[&str], actor: Actor, primary: &str) -> String {
    let mut words = vec!["adr", cmd];
    words.extend_from_slice(args);
    let into = Act::ChangesInto(primary.to_string());
    let again = Act::Runs(command(&words));
    let next = match actor {
        Actor::Agent | Actor::Builder => Next::agent(into).then(again),
        Actor::Owner => Next::owner(into).then(again),
    };
    refuse(format!("{}: the record is at {}", cmd, primary), next)
}

/// A command run in a satellite whose primary holds no record at `primary`
/// (per "What does check in a satellite read the record from?").
pub fn record_missing(cmd: &str, primary: &str) -> String {
    refuse(
        format!("{}: record not found at {}", cmd, primary),
        Next::agent(Act::Restores(format!(
            "the primary's checkout at {}",
            primary
        ))),
    )
}

/// `check` in the primary: the satellite listed at `path` has no checkout
/// there (per "What does check in the primary report for a satellite missing
/// at its path?").
pub fn satellite_missing(path: &str, url: &str) -> Said {
    if url.is_empty() {
        said(
            &crate::rule::SATELLITE_FOUND_AT_PATH,
            format!("satellite {} not found, no url", path),
            Next::agent(Act::Restores(path.to_string())),
        )
    } else {
        said(
            &crate::rule::SATELLITE_FOUND_AT_PATH,
            format!("satellite {} not found, url {}", path, url),
            Next::agent_runs(command(&["git", "clone", url, path])),
        )
    }
}

/// `check` in a satellite: its brief differs from the primary's.
pub fn satellite_brief_differs() -> Said {
    said(
        &crate::rule::SATELLITE_BRIEF_MATCHES_PRIMARY,
        "adrdom/state/brief.md differs from the primary's",
        Next::agent_runs(Cmd::fixed("adr install")),
    )
}

/// `check` in a satellite: it has no brief.
pub fn satellite_brief_missing() -> Said {
    said(
        &crate::rule::SATELLITE_BRIEF_MATCHES_PRIMARY,
        "adrdom/state/brief.md is missing",
        Next::agent_runs(Cmd::fixed("adr install")),
    )
}

/// `check` in a satellite: its terms brief differs from the primary's terms
/// file.
pub fn satellite_terms_brief_differs() -> Said {
    said(
        &crate::rule::TERMS_BRIEF_MATCHES_TERMS,
        format!(
            "{} differs from the primary's {}",
            crate::repo::TERMS_BRIEF_PATH,
            crate::repo::TERMS_PATH
        ),
        Next::agent_runs(Cmd::fixed("adr install")),
    )
}

/// `check` in a satellite: it has no terms brief.
pub fn satellite_terms_brief_missing() -> Said {
    said(
        &crate::rule::TERMS_BRIEF_MATCHES_TERMS,
        format!("{} missing", crate::repo::TERMS_BRIEF_PATH),
        Next::agent_runs(Cmd::fixed("adr install")),
    )
}

/// `install --satellite <path>`: nothing there is a git repository.
pub fn satellite_not_a_repo(given: &str) -> String {
    refuse(
        format!("install: {} is not a git repository", given),
        Next::agent(Act::Rewrites("the path after --satellite".to_string())),
    )
}

/// `install --satellite <path>`: the path holds a record of its own, or is
/// the primary itself.
pub fn satellite_holds_record(given: &str) -> String {
    refuse(
        format!("install: {} holds a record of its own", given),
        Next::agent(Act::Rewrites("the path after --satellite".to_string())),
    )
}

/// `install --satellite <path>` in a repository with no record to share.
pub fn satellite_without_record(given: &str) -> String {
    refuse(
        "install: this repository holds no record".to_string(),
        Next::agent_runs(Cmd::fixed("adr install")).then(Act::Runs(command(&[
            "adr",
            "install",
            "--satellite",
            given,
        ]))),
    )
}

/// `install --satellite <path>`: the satellite is held by another primary,
/// at `other` (per "Does install take a satellite that holds the path of
/// another primary?", 490cdbb6-0482-41ba-8f57-36a6136b166f A). The owner
/// uninstalls in the satellite, then the agent installs it here.
pub fn satellite_of_another(given: &str, satellite: &str, other: &str) -> String {
    refuse(
        format!("install: {} is held by the primary at {}", given, other),
        Next::all(vec![
            Next::owner_then_runs(
                Act::ChangesInto(satellite.to_string()),
                Cmd::fixed("adr uninstall"),
            ),
            Next::agent_runs(command(&["adr", "install", "--satellite", given])),
        ]),
    )
}

/// `install --claude`, `--codex` or `--planning` in a satellite.
pub fn satellite_takes_no_flag(flag: &str) -> String {
    refuse(
        format!("install: a satellite takes no {}", flag),
        Next::agent_runs(Cmd::fixed("adr install")),
    )
}

/// The first line of `show` in a satellite: the directory the record was
/// read from (per "Does show in a satellite name its primary?" and "Does
/// show in a satellite print the directory it read the record from?",
/// 7e248514-25f8-46ed-936e-2849cd27cf86 A).
pub fn record_line(primary: &str) -> String {
    format!("record: {}", primary)
}
