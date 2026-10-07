use std::fs;
use std::io::Write;
use std::path::Path;

pub const EVENTS: [&str; 14] = [
    "asked",
    "answered",
    "refused",
    "superseded",
    "withdrawn",
    "approved",
    "context",
    "terms",
    "term",
    "banned",
    "unbanned",
    "assumes",
    "blame",
    "dropped",
];

#[derive(Debug, Clone)]
pub struct Row {
    pub time: String,
    pub event: String,
    pub id: String,
    pub hash: String,
    pub detail: String,
    /// The tool version that wrote this row (src/version.rs's `VERSION`
    /// at write time). Empty for a row read from an older 5-field line, from
    /// before "Does every ledger row carry the tool version?" added this
    /// field; `read` fills that in as `String::new()` rather than refusing it.
    pub version: String,
}

pub struct Ledger {
    pub rows: Vec<Row>,
    pub malformed: Vec<(usize, String)>,
}

pub fn now_rfc3339() -> String {
    use time::format_description::well_known::Rfc3339;
    use time::OffsetDateTime;
    let now = OffsetDateTime::now_utc().replace_nanosecond(0).unwrap();
    now.format(&Rfc3339).unwrap()
}

/// The time a command's rows carry (per "Does a command write a time later
/// than the last ledger row?", aefa7f3c-70c3-412e-8d73-16ff9e2b703a A): the
/// later of the clock (`now_rfc3339`, whole seconds) and one second past
/// `led`'s own latest row -- `rows` is already ordered by time, so that's
/// just the last one. Called once per command run, before any row is
/// built, and the one value it returns is reused for every row that run
/// writes, however many separate `append` calls the run makes.
pub fn write_time(led: &Ledger) -> String {
    use time::format_description::well_known::Rfc3339;
    use time::{Duration, OffsetDateTime};
    let now = OffsetDateTime::now_utc().replace_nanosecond(0).unwrap();
    let floor = led
        .rows
        .last()
        .and_then(|r| parse_row_time(&r.time))
        .map(|t| t + Duration::seconds(1));
    let chosen = match floor {
        Some(f) if f > now => f,
        _ => now,
    };
    chosen.format(&Rfc3339).unwrap()
}

pub fn read(path: &Path) -> Ledger {
    parse(&fs::read_to_string(path).unwrap_or_default())
}

/// A row's `time` field, parsed as RFC3339 -- `None` when it doesn't
/// parse, which `parse` below treats the same as any other malformed row
/// rather than sorting it in as though its time were somehow comparable.
fn parse_row_time(s: &str) -> Option<time::OffsetDateTime> {
    use time::format_description::well_known::Rfc3339;
    time::OffsetDateTime::parse(s, &Rfc3339).ok()
}

/// Orders `rows` (each paired with the exact line it was read from) by
/// time (per "Does the tool take the order of ledger rows from their
/// time?"): the parsed `time` first, and -- since two rows can share a
/// timestamp when a run writes more than one in the same second -- the
/// row's own raw line text as the tie-break, never the position the row
/// happened to occupy in the file. A stable sort, so two rows that are
/// byte-identical (same time, same line) keep whatever relative order they
/// were read in, which is the only case left where that can matter.
fn order_by_time(mut parsed: Vec<(time::OffsetDateTime, String, Row)>) -> Vec<Row> {
    parsed.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    parsed.into_iter().map(|(_, _, r)| r).collect()
}

/// Parses ledger rows straight from `content`, without touching the
/// filesystem: what `read` does after its own `fs::read_to_string`, split
/// out so a caller that already has the bytes -- `check`, reading through
/// `repo::Source` so a staged ledger edit is judged too -- doesn't have to
/// write them to a temp file first.
///
/// `rows` comes back ordered by time (per "Does the tool take the order of
/// ledger rows from their time?"), not by the rows' place in `content`, so
/// no lookup that treats the last matching row in `rows` as "the latest"
/// needs to know or care where in the file a row actually sits.
/// `malformed`'s line numbers still name `content`'s real line numbers,
/// computed in this same pass before the reordering happens.
pub fn parse(content: &str) -> Ledger {
    let mut parsed: Vec<(time::OffsetDateTime, String, Row)> = Vec::new();
    let mut malformed = Vec::new();
    for (idx, line) in content.lines().enumerate() {
        if line.is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split('\t').collect();
        let version = match parts.len() {
            5 => String::new(),
            6 => parts[5].to_string(),
            _ => {
                malformed.push((idx + 1, line.to_string()));
                continue;
            }
        };
        if !EVENTS.contains(&parts[1]) {
            malformed.push((idx + 1, line.to_string()));
            continue;
        }
        let Some(time) = parse_row_time(parts[0]) else {
            malformed.push((idx + 1, line.to_string()));
            continue;
        };
        parsed.push((
            time,
            line.to_string(),
            Row {
                time: parts[0].to_string(),
                event: parts[1].to_string(),
                id: parts[2].to_string(),
                hash: parts[3].to_string(),
                detail: parts[4].to_string(),
                version,
            },
        ));
    }
    let rows = order_by_time(parsed);
    Ledger { rows, malformed }
}

/// Always 6 tab-separated fields (per "Does every ledger row carry the
/// tool version?"): the pre-existing 5 plus
/// `version`, even when it's empty.
pub fn row_line(r: &Row) -> String {
    let detail = r.detail.replace(['\t', '\n'], " ");
    let version = r.version.replace(['\t', '\n'], " ");
    format!(
        "{}\t{}\t{}\t{}\t{}\t{}",
        r.time, r.event, r.id, r.hash, detail, version
    )
}

pub fn append(path: &Path, rows: &[Row]) -> std::io::Result<()> {
    if rows.is_empty() {
        return Ok(());
    }
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    for r in rows {
        writeln!(f, "{}", row_line(r))?;
    }
    Ok(())
}

/// The events that put a question in a state (`Ledger::latest_state`): the
/// one list every reader of a question's state reads.
pub const STATE_EVENTS: [&str; 6] = ["asked", "answered", "refused", "superseded", "withdrawn", "dropped"];

impl Ledger {
    /// The latest row that puts an id in a state: `asked`, `answered`,
    /// `refused`, `superseded`, `withdrawn`, or `dropped` are all state rows;
    /// the most recent one wins. A `dropped` row ends an id for good (per
    /// "Does answer remove a question whose answer set lacks the answer?"):
    /// the question left its file, asked or not, and never returns.
    pub fn latest_state(&self, id: &str) -> Option<&Row> {
        self.rows.iter().rfind(|r| r.id == id && STATE_EVENTS.contains(&r.event.as_str()))
    }

    /// Every identifier a state row names, each once, sorted: the
    /// questions the ledger holds a state for.
    pub fn state_ids(&self) -> std::collections::BTreeSet<String> {
        self.rows.iter().filter(|r| STATE_EVENTS.contains(&r.event.as_str())).map(|r| r.id.clone()).collect()
    }

    pub fn latest_asked(&self, id: &str) -> Option<&Row> {
        self.rows.iter().rfind(|r| r.id == id && r.event == "asked")
    }

    /// The position in `rows` -- ordered by time, per "Does the tool take
    /// the order of ledger rows from their time?", never by a row's place
    /// in the file -- of the latest `asked` row for `id`, if any (per "Does
    /// one decision supersede several?"). A supersedes freeze check compares
    /// this position rather than the `time` string only because two rows
    /// can carry the exact same time (every row one command run writes now
    /// shares one time, per "Does a command write a time later than the
    /// last ledger row?"); the id's own `asked` row and the target's
    /// `superseded` row are always written by two different command runs
    /// (`ask` and `answer`), so in practice their times already differ and
    /// this position agrees with the time order -- but the position is what
    /// the comparison actually reads.
    pub fn latest_asked_pos(&self, id: &str) -> Option<usize> {
        self.rows.iter().rposition(|r| r.id == id && r.event == "asked")
    }

    /// The position in `rows` -- ordered by time, never by file place, same
    /// as `latest_asked_pos` -- of the latest `superseded` row for `id`, if
    /// any (per "Does one decision supersede several?"). Paired with
    /// `latest_asked_pos` to tell whether a supersedes target was
    /// superseded before or after a question asking it was itself asked.
    pub fn latest_superseded_pos(&self, id: &str) -> Option<usize> {
        self.rows.iter().rposition(|r| r.id == id && r.event == "superseded")
    }

    /// The latest row of `event` whose `detail` column equals `path` and
    /// whose id is `-`: the file's own row, never one of the per-question
    /// `approved` rows that name the same path.
    pub fn latest_for_path(&self, event: &str, path: &str) -> Option<&Row> {
        self.rows
            .iter()
            .rfind(|r| r.event == event && r.detail == path && r.id == "-")
    }

    /// The latest row of `event` for the identifier `id`.
    pub fn latest_for_id(&self, event: &str, id: &str) -> Option<&Row> {
        self.rows.iter().rfind(|r| r.event == event && r.id == id)
    }

    /// Every `assumes` row for `id` (the asked question's id), one per
    /// dependent heading (the row's `detail`), the latest row winning when
    /// a dependent's set was re-asked with a different hash. Returned in
    /// first-seen order.
    pub fn assumes_for(&self, id: &str) -> Vec<&Row> {
        let mut order: Vec<String> = Vec::new();
        let mut latest: std::collections::HashMap<String, &Row> = std::collections::HashMap::new();
        for r in &self.rows {
            if r.event == "assumes" && r.id == id {
                if !latest.contains_key(&r.detail) {
                    order.push(r.detail.clone());
                }
                latest.insert(r.detail.clone(), r);
            }
        }
        order.into_iter().filter_map(|d| latest.get(&d).copied()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn temp_path() -> std::path::PathBuf {
        std::env::temp_dir().join(format!("adrdom-ledger-test-{}.tsv", Uuid::new_v4()))
    }

    fn row(event: &str, id: &str, hash: &str, detail: &str, version: &str) -> Row {
        Row {
            time: "2024-01-01T00:00:00Z".to_string(),
            event: event.to_string(),
            id: id.to_string(),
            hash: hash.to_string(),
            detail: detail.to_string(),
            version: version.to_string(),
        }
    }

    #[test]
    fn read_accepts_5_field_rows_with_empty_version() {
        let path = temp_path();
        fs::write(&path, "2024-01-01T00:00:00Z\tasked\tid-1\thash-1\theading\n").unwrap();
        let led = read(&path);
        assert!(led.malformed.is_empty(), "{:?}", led.malformed);
        assert_eq!(led.rows.len(), 1);
        assert_eq!(led.rows[0].version, "");
        fs::remove_file(&path).ok();
    }

    #[test]
    fn read_accepts_6_field_rows_with_version() {
        let path = temp_path();
        fs::write(
            &path,
            "2024-01-01T00:00:00Z\tasked\tid-1\thash-1\theading\t0.1.0\n",
        )
        .unwrap();
        let led = read(&path);
        assert!(led.malformed.is_empty(), "{:?}", led.malformed);
        assert_eq!(led.rows.len(), 1);
        assert_eq!(led.rows[0].version, "0.1.0");
        fs::remove_file(&path).ok();
    }

    #[test]
    fn a_row_with_the_wrong_field_count_is_malformed() {
        let path = temp_path();
        fs::write(
            &path,
            "2024-01-01T00:00:00Z\tasked\tid-1\thash-1\theading\t0.1.0\textra\n",
        )
        .unwrap();
        let led = read(&path);
        assert!(led.rows.is_empty());
        assert_eq!(led.malformed.len(), 1);
        fs::remove_file(&path).ok();
    }

    #[test]
    fn row_line_always_writes_six_fields() {
        let r = row("asked", "id-1", "hash-1", "heading", "0.1.0");
        let line = row_line(&r);
        assert_eq!(line.split('\t').count(), 6, "{}", line);

        let r5 = row("asked", "id-1", "hash-1", "heading", "");
        let line5 = row_line(&r5);
        assert_eq!(line5.split('\t').count(), 6, "{}", line5);
        assert!(line5.ends_with('\t'), "{}", line5);
    }

    #[test]
    fn refused_and_assumes_are_recognized_events() {
        assert!(EVENTS.contains(&"refused"));
        assert!(EVENTS.contains(&"assumes"));

        let path = temp_path();
        fs::write(
            &path,
            "2024-01-01T00:00:00Z\trefused\tid-1\thash-1\theading\t0.1.0\n\
             2024-01-01T00:00:01Z\tassumes\tid-1\thash-2\tdependent heading\t0.1.0\n",
        )
        .unwrap();
        let led = read(&path);
        assert!(led.malformed.is_empty(), "{:?}", led.malformed);
        assert_eq!(led.rows.len(), 2);
        fs::remove_file(&path).ok();
    }

    #[test]
    fn latest_state_considers_refused() {
        let led = Ledger {
            rows: vec![
                row("asked", "id-1", "h1", "Q?", "0.1.0"),
                row("refused", "id-1", "h1", "Q?", "0.1.0"),
            ],
            malformed: vec![],
        };
        let latest = led.latest_state("id-1").unwrap();
        assert_eq!(latest.event, "refused");
    }

    #[test]
    fn assumes_for_returns_the_latest_row_per_dependent() {
        let led = Ledger {
            rows: vec![
                row("asked", "id-1", "h1", "Q?", "0.1.0"),
                row("assumes", "id-1", "hash-a", "Dependent A?", "0.1.0"),
                row("assumes", "id-1", "hash-b", "Dependent B?", "0.1.0"),
                // Q re-asked with a new set on Dependent A: the latest row
                // for that dependent wins.
                row("assumes", "id-1", "hash-a2", "Dependent A?", "0.1.0"),
            ],
            malformed: vec![],
        };
        let rows = led.assumes_for("id-1");
        assert_eq!(rows.len(), 2);
        let a = rows.iter().find(|r| r.detail == "Dependent A?").unwrap();
        assert_eq!(a.hash, "hash-a2");
        let b = rows.iter().find(|r| r.detail == "Dependent B?").unwrap();
        assert_eq!(b.hash, "hash-b");
    }

    #[test]
    fn latest_asked_pos_and_latest_superseded_pos_use_ledger_order_not_timestamps() {
        // Two rows sharing a timestamp: position in the vec (append order)
        // is still what breaks the tie, not the (equal) time string.
        let led = Ledger {
            rows: vec![
                row("asked", "q1", "h1", "Q1?", "0.1.0"),
                row("asked", "q2", "h2", "Q2?", "0.1.0"),
                row("superseded", "x", "-", "q1", "0.1.0"),
            ],
            malformed: vec![],
        };
        assert_eq!(led.latest_asked_pos("q1"), Some(0));
        assert_eq!(led.latest_asked_pos("q2"), Some(1));
        assert_eq!(led.latest_superseded_pos("x"), Some(2));
        assert_eq!(led.latest_superseded_pos("nope"), None);
        // "x" was superseded after q2's asked row but not needed here --
        // the position ordering itself is what this test checks.
        assert!(led.latest_superseded_pos("x") > led.latest_asked_pos("q2"));
    }

    #[test]
    fn assumes_for_ignores_other_ids_and_other_events() {
        let led = Ledger {
            rows: vec![
                row("assumes", "id-1", "h", "A?", "0.1.0"),
                row("assumes", "id-2", "h", "B?", "0.1.0"),
                row("asked", "id-1", "h", "A?", "0.1.0"),
            ],
            malformed: vec![],
        };
        let rows = led.assumes_for("id-1");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].detail, "A?");
    }

    // adr: 385a57d5-5f76-454a-ae5d-9b0d6f5c1492
    //
    // "the latest row is the one with the latest time, and the place of a
    // row in the file carries no meaning": `parse` itself hands back
    // `rows` in time order, not file order, when the file's own line order
    // disagrees with it.
    #[test]
    fn parse_orders_rows_by_time_not_by_file_position() {
        let content = "2024-01-01T00:00:03Z\tasked\tid-1\th1\tlater\t0.1.0\n\
                        2024-01-01T00:00:01Z\tasked\tid-2\th2\tearliest\t0.1.0\n\
                        2024-01-01T00:00:02Z\tasked\tid-3\th3\tmiddle\t0.1.0\n";
        let led = parse(content);
        assert!(led.malformed.is_empty(), "{:?}", led.malformed);
        let details: Vec<&str> = led.rows.iter().map(|r| r.detail.as_str()).collect();
        assert_eq!(details, vec!["earliest", "middle", "later"]);
    }

    // A row whose time doesn't parse as RFC3339 is handled the same way any
    // other malformed row already is: it lands in `malformed` (at its real
    // file line number), and is left out of `rows` -- never sorted in as
    // though it had some fallback time.
    #[test]
    fn parse_treats_an_unparseable_time_as_malformed() {
        let content = "not-a-time\tasked\tid-1\th1\theading\t0.1.0\n\
                        2024-01-01T00:00:01Z\tasked\tid-2\th2\tgood row\t0.1.0\n";
        let led = parse(content);
        assert_eq!(led.rows.len(), 1);
        assert_eq!(led.rows[0].detail, "good row");
        assert_eq!(led.malformed.len(), 1);
        assert_eq!(led.malformed[0].0, 1, "the bad row is the file's real line 1");
        assert!(led.malformed[0].1.starts_with("not-a-time"));
    }

    // Two rows sharing the exact same time are ordered by their own raw
    // line text next, never by which one the file happened to write
    // first: swapping their file positions must not change the tie-break.
    #[test]
    fn parse_breaks_a_time_tie_by_row_text_not_file_position() {
        let a = "2024-01-01T00:00:00Z\tasked\tid-1\th1\taaa\t0.1.0\n";
        let b = "2024-01-01T00:00:00Z\tasked\tid-2\th2\tbbb\t0.1.0\n";

        let forward = parse(&format!("{}{}", a, b));
        let backward = parse(&format!("{}{}", b, a));

        let forward_details: Vec<&str> = forward.rows.iter().map(|r| r.detail.as_str()).collect();
        let backward_details: Vec<&str> = backward.rows.iter().map(|r| r.detail.as_str()).collect();
        assert_eq!(forward_details, vec!["aaa", "bbb"]);
        assert_eq!(backward_details, vec!["aaa", "bbb"], "file order must not change the tie-break");
    }
}
