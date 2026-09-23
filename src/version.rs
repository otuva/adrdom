//! The crate's version, read from the repo-root `VERSION` file so `adr
//! --version` and the ledger's `version` column (src/ledger.rs) agree with
//! a single source of truth instead of drifting from Cargo.toml (8.1/VERSION
//! note, batch 8).

const RAW: &str = include_str!("../VERSION");

/// `str::trim` is not a `const fn`, so the trailing newline the `VERSION`
/// file carries is stripped by hand, byte-wise, at compile time. Strips one
/// trailing `\n`, then one trailing `\r` after that (a CRLF `VERSION`
/// file), so either line ending leaves the same bare version string.
const fn trim_trailing_newline(s: &str) -> &str {
    let bytes = s.as_bytes();
    let len = bytes.len();
    let end = if len > 0 && bytes[len - 1] == b'\n' { len - 1 } else { len };
    let end = if end > 0 && bytes[end - 1] == b'\r' { end - 1 } else { end };
    let (head, _) = bytes.split_at(end);
    match core::str::from_utf8(head) {
        Ok(s2) => s2,
        Err(_) => s,
    }
}

pub const VERSION: &str = trim_trailing_newline(RAW);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_matches_cargo_pkg_version() {
        assert_eq!(env!("CARGO_PKG_VERSION"), VERSION);
    }

    #[test]
    fn trim_trailing_newline_strips_lf_crlf_and_bare() {
        assert_eq!(trim_trailing_newline("0.1.0\r\n"), "0.1.0");
        assert_eq!(trim_trailing_newline("0.1.0\n"), "0.1.0");
        assert_eq!(trim_trailing_newline("0.1.0"), "0.1.0");
    }
}
