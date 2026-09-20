# adr

A control layer between AI coding agents and a repository. Decisions are
questions in markdown records under `adrdom/decisions/`, answered only by the
repository's owner, and recorded in an append-only ledger at
`adrdom/state/ledger.tsv`; a separate check requires every in-force decision
to be tied to an `adr: <uuid>` reference in the code or explicitly listed as
unchecked.

## Build

```sh
cargo build --release
```

The binary is a single static-ish executable at `target/release/adr` with no
runtime dependencies beyond `git` on `PATH`.

## Commands

- `adr init` — set up `adrdom/decisions/`, `adrdom/state/`, git hooks, and
  `.editorconfig` in the current repository; also merges a set of deny
  entries into `.claude/settings.json`'s `permissions.deny`. Idempotent.
- `adr ask <record>` — assign ids to new questions, write their `terms:`
  binding line, and record them as asked (re-asking ones that changed since
  they were last asked); refuses on an unapproved plan/context/terms file, an
  unanswered question in an earlier group, a banned word, or a retired term.
- `adr lock <id> <letter>` — owner-only. Answer a question by id and letter;
  writes the `**Answer:**` line and an `answered` ledger row, plus one
  `superseded` row per id named in the question's `supersedes:` line.
- `adr approve <file>` — owner-only. For a record: approve its current plan
  and/or context (an `approved`/`context` ledger row each). For
  `adrdom/terms.md`: fill in missing term ids and approve it (a `terms` row),
  refusing first on a shape error, a binding that would resolve differently,
  or a newly banned word appearing in a record.
- `adr unban` — owner-only. Read lines from stdin and remove them (and
  their tokens) from the ban list.
- `adr ban` — read lines from stdin and ban them (and their tokens) from
  ever appearing in the repository or a commit message.
- `adr show [record]` — print every decision currently in force (its
  heading and the sentence of the chosen option).
- `adr serve [--port N]` — owner-only. Serve the owner's page on
  `127.0.0.1` (default port 7357), gated by a per-start token printed to
  stdout; shows status, open questions, and decisions, with forms to
  approve, lock, and unban.
- `adr check [--staged] [--message <path>]` — run every governance check;
  used by `.githooks/pre-commit` and `.githooks/commit-msg`.

`adr lock`, `adr approve`, `adr unban`, and `adr serve` refuse to run unless
invoked interactively by a human (a real TTY, and no `CLAUDECODE` in the
environment) — an AI agent cannot answer a question, approve anything, edit
the ban list, or start the server.

## Record shape

A file under `adrdom/decisions/` starts with a first non-blank line `# <topic>`
(exactly one h1), followed by context prose (any text) up to the first `## `
line. `## <text>` lines are groups, in asking order (the first group is the
lowest tier: an earlier group's questions must all be answered before a later
group's are asked). Each group holds `### <text>?` questions (h3, must end in
`?`) or principle blocks `**Pn. sentence**`, in the group they follow.

A question/principle body holds only: blank lines; `id: `, `supersedes: `,
and `terms: ` lines; one `Problem: ` and one `Standard: ` line; option lines
`- X) sentence.` with optional two-space-indented `  Why: ` / `  Costs: `
lines under them; and a trailing `**Answer:** <letter>`. Anything else is a
shape error. A **skeleton heading** — one with no non-blank line before the
next heading — has no id and no body, and is never a failure.

## Terms

`adrdom/terms.md` starts with `# Terms`, followed by `## <name>` entries, each
holding `id:`, `definition:`, `test:`, `owner:` (a record path), `aliases:`,
`banned:` (comma-separated, may be empty), and an optional `retired: yes`
line. Nothing else but blank lines is allowed. `adr ask` resolves a
question's text against every term's name/alias (whole word, case-insensitive,
longest match first) and writes the result under the question's `id: ` line
as `terms: <name>=<uuid>, <name>=<uuid>`.

## Install and check

`adr init` merges a set of `Bash`/`Edit`/`Write` deny entries into
`.claude/settings.json`'s `permissions.deny` (covering `adr lock`,
`adr approve`, `adr unban`, `adr serve`, and edits to `adrdom/state/`).
`adr check`'s `install` check fails if any entry is missing, naming
`adr init` as the fix.

## Ledger

`adrdom/state/ledger.tsv` is append-only, one tab-separated
`time, event, id, hash, detail` row per event: `asked`, `answered`,
`superseded`, and `withdrawn` track a question; `approved` and `context`
track a record's plan and context prose; `terms` tracks the terms file;
`banned` and `unbanned` track the ban list.

## Tests

```sh
cargo test --all-features
```

`owner-bypass` is a test-only cargo feature (gated further by
`ADR_OWNER_BYPASS=1`) that lets the integration test suite exercise
`lock`/`approve`/`unban` non-interactively. It is not part of the default
feature set and must not be enabled in a release build.
