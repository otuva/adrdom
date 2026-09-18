# adr

A control layer between AI coding agents and a repository. Decisions are
questions in markdown records under `adrdom/decisions/`, answered only by the
repository's owner, recorded in an append-only ledger at
`adrdom/state/ledger.tsv`, and tied to a test (or explicitly listed as
unchecked) before they count as in force.

## Build

```sh
cargo build --release
```

The binary is a single static-ish executable at `target/release/adr` with no
runtime dependencies beyond `git` on `PATH`.

## Commands

- `adr init` — set up `adrdom/decisions/`, `adrdom/state/`, git hooks, and
  `.editorconfig` in the current repository. Idempotent.
- `adr ask <record>` — assign ids to new questions and record them as asked;
  re-asks questions that changed since they were last asked.
- `adr lock <id> <letter>` — owner-only. Answer a question by id.
- `adr show [record]` — print every decision currently in force.
- `adr ban` — read lines from stdin and ban them (and their tokens) from
  ever appearing in the repository or a commit message.
- `adr override <check> <id-or-dash> --reason <text>` — owner-only. Accept a
  named check's failure for now, with a reason on record.
- `adr check [--staged] [--message <path>]` — run every governance check;
  used by `.githooks/pre-commit` and `.githooks/commit-msg`.

`adr lock` and `adr override` refuse to run unless invoked interactively by
a human (a real TTY, and no `CLAUDECODE` in the environment) — an AI agent
cannot answer a question or wave off a failing check.

## Tests

```sh
cargo test --features owner-bypass
```

`owner-bypass` is a test-only cargo feature (gated further by
`ADR_OWNER_BYPASS=1`) that lets the integration test suite exercise
`lock`/`override` non-interactively. It is not part of the default feature
set and must not be enabled in a release build.
