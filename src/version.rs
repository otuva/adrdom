//! The crate's version, read from the repo-root `VERSION` file so `adr
//! --version` and the ledger's `version` column (src/ledger.rs) agree with
//! a single source of truth instead of drifting from Cargo.toml (8.1/VERSION
//! note, batch 8).

const RAW: &str = include_str!("../VERSION");

/// `str::trim` is not a `const fn`, so the single trailing newline the
/// `VERSION` file carries is stripped by hand, byte-wise, at compile time.
const fn trim_trailing_newline(s: &str) -> &str {
    let bytes = s.as_bytes();
    let len = bytes.len();
    if len > 0 && bytes[len - 1] == b'\n' {
        let (head, _) = bytes.split_at(len - 1);
        match core::str::from_utf8(head) {
            Ok(s2) => s2,
            Err(_) => s,
        }
    } else {
        s
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
}
