# Contributing

This repository is governed by `adr`. What the code does is a decision the owner answered.

1. Read `adrdom/state/brief.md`, then run `adr show`.
2. A change that follows a current decision: open a pull request with a test that carries `adr: <id>`.
3. A behaviour no decision covers: write the question in the shape the brief gives and open an issue with it. A pull request for it is closed as a finding.
4. Before pushing: `cargo test --all-features && adr check`.

Commit messages: imperative, present tense, prefixed `feat`, `fix`, `refactor`, `test`, `docs`, `chore` or `infra`.
