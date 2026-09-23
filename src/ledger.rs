use std::fs;
use std::io::Write;
use std::path::Path;

pub const EVENTS: [&str; 11] = [
    "asked",
    "answered",
    "refused",
    "superseded",
    "withdrawn",
    "approved",
    "context",
    "terms",
    "banned",
    "unbanned",
    "assumes",
];

#[derive(Debug, Clone)]
pub struct Row {
    pub time: String,
    pub event: String,
    pub id: String,
    pub hash: String,
    pub detail: String,
    /// The tool version that wrote this row (src/version.rs's `VERSION`
    /// at write time). Empty for a row read from a pre-8.1 5-field line;
    /// `read` fills that in as `String::new()` rather than refusing it.
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

pub fn read(path: &Path) -> Ledger {
    let content = fs::read_to_string(path).unwrap_or_default();
    let mut rows = Vec::new();
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
        rows.push(Row {
            time: parts[0].to_string(),
            event: parts[1].to_string(),
            id: parts[2].to_string(),
            hash: parts[3].to_string(),
            detail: parts[4].to_string(),
            version,
        });
    }
    Ledger { rows, malformed }
}

/// Always 6 tab-separated fields (8.1 A): the pre-existing 5 plus
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

impl Ledger {
    /// The latest row that puts an id in a state: `asked`, `answered`,
    /// `refused`, `superseded`, or `withdrawn` are all state rows; the most
    /// recent one wins.
    pub fn latest_state(&self, id: &str) -> Option<&Row> {
        self.rows.iter().rfind(|r| {
            r.id == id
                && matches!(
                    r.event.as_str(),
                    "asked" | "answered" | "refused" | "superseded" | "withdrawn"
                )
        })
    }

    pub fn latest_asked(&self, id: &str) -> Option<&Row> {
        self.rows.iter().rfind(|r| r.id == id && r.event == "asked")
    }

    /// The latest row of `event` whose `detail` column equals `path`.
    pub fn latest_for_path(&self, event: &str, path: &str) -> Option<&Row> {
        self.rows
            .iter()
            .rfind(|r| r.event == event && r.detail == path)
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
}
