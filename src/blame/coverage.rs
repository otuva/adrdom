//! The check of a blame file's ranges against the length of its origin.
use super::Row;

/// A row whose range fails to fit inside the origin's `1..=n` (too far, or
/// -- since parsing already rejects `end < start` -- never inverted): named
/// separately from a gap so `approve` can name the excess distinctly.
pub struct BeyondError {
    pub start: usize,
    pub end: usize,
}

/// One run of consecutive lines (1-based, inclusive) covered by no row.
pub struct GapError {
    pub start: usize,
    pub end: usize,
}

/// Checks every row's range against the origin's line count `n`
/// (`git cat-file -p <blob> | wc -l`, trailing newline not counted, per
/// "Does a blame file cover its whole origin?"). A range wholly or partly
/// beyond `n` is reported by `beyond`; the rest are used to mark
/// `1..=n` covered, and every run left uncovered is reported by `gaps`.
/// Overlap is never an error (5c399a14 A: "a range may appear in more than
/// one row").
pub fn coverage_errors(rows: &[Row], n: usize) -> (Vec<BeyondError>, Vec<GapError>) {
    let mut beyond = Vec::new();
    let mut covered = vec![false; n + 1]; // 1-based; index 0 unused.
    for r in rows {
        if r.start > n || r.end > n {
            beyond.push(BeyondError {
                start: r.start,
                end: r.end,
            });
        }
        // Only the part inside the origin marks coverage.
        let end = r.end.min(n);
        if r.start <= end {
            covered[r.start..=end].fill(true);
        }
    }

    let mut gaps = Vec::new();
    let mut i = 1;
    while i <= n {
        if covered[i] {
            i += 1;
            continue;
        }
        let start = i;
        while i <= n && !covered[i] {
            i += 1;
        }
        gaps.push(GapError { start, end: i - 1 });
    }
    (beyond, gaps)
}

#[cfg(test)]
mod tests {
    use super::*;

    // adr: 5c399a14-1e5c-48eb-b502-b57524c01c24
    #[test]
    fn coverage_errors_finds_a_gap_and_an_overlap_passes() {
        let row = |start, end| Row {
            start,
            end,
            value: "detail".into(),
        };
        let rows = vec![
            row(1, 2),
            row(2, 4),
            // line 5 is a gap.
        ];
        let (beyond, gaps) = coverage_errors(&rows, 6);
        assert!(beyond.is_empty());
        assert_eq!(gaps.len(), 1);
        assert_eq!((gaps[0].start, gaps[0].end), (5, 6));
    }

    // adr: 5c399a14-1e5c-48eb-b502-b57524c01c24
    #[test]
    fn coverage_errors_finds_a_range_beyond_n() {
        let rows = vec![Row {
            start: 1,
            end: 10,
            value: "decision".into(),
        }];
        let (beyond, gaps) = coverage_errors(&rows, 5);
        assert_eq!(beyond.len(), 1);
        assert_eq!((beyond[0].start, beyond[0].end), (1, 10));
        assert!(
            gaps.is_empty(),
            "the part inside the origin counts as covered"
        );
    }
}
