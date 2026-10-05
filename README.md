# adr

A control layer between AI coding agents and a repository. Decisions are
questions in markdown topic files under `adrdom/decisions/`, answered only by the
repository's owner, and recorded in an append-only ledger at
`adrdom/state/ledger.tsv`; a separate check reports a current decision with no
`adr: <uuid>` reference in the code as untested, in one line per decision, and
fails a reference to a decision that is not current.

## Build

```sh
cargo build --release
```

The binary is a single static-ish executable at `target/release/adr` with no
runtime dependencies beyond `git` on `PATH`.

## Commands

- `adr install` — set up `adrdom/decisions/`, `adrdom/state/`, and git hooks
  in the current repository (writing or rewriting each hook to match what
  it writes, and leaving an already-current one untouched), and write the
  agent brief into AGENTS.md (the
  file agents read) between marker lines, refreshing it in place on a later
  run. Idempotent. `adr install --claude` additionally creates CLAUDE.md
  holding a line importing AGENTS.md when the file is missing (or adds that
  import to an existing CLAUDE.md), and merges a set of deny entries into
  `.claude/settings.json`'s `permissions.deny`. `adr install --codex`
  additionally writes `.codex/rules/adrdom.rules`, forbidding the owner's
  commands under Codex's project rules. Not owner-only.
- `adr uninstall` — owner-only. Removes everything `adr install` wrote
  outside `adrdom/`: the marked brief section from AGENTS.md (deleting the
  file if nothing else is in it), the AGENTS.md import it added to
  CLAUDE.md (deleting that file too if nothing else is in it), the git hooks and `core.hooksPath` (only if unchanged since
  install), the deny entries from `.claude/settings.json`, and
  `.codex/rules/adrdom.rules` (only if unchanged since install; removing the
  now-empty `.codex/rules` and `.codex` directories too). Leaves
  `adrdom/` untouched. Idempotent.
- `adr ask <topic_file>` — write each new question's `terms:` binding line
  and record it as asked (re-asking ones that changed since they were last
  asked); refuses on an unapproved plan/context/terms file, a
  question outside the open tier (the lowest tier with an unanswered
  question), a tier over the cap, a question standing on a refused question,
  a question left moot by an answer already given, a tier with a depends
  line into it that carries no answer set, a banned word, or a retired term.
- `adr answer <id>@<version> <letter>` — owner-only. Refuses while
  `adrdom/terms.md` is unapproved; otherwise answers a question by id
  and letter, writing the `**Answer:**` line and an `answered` ledger row,
  plus one `superseded` row per id named in the question's `supersedes:`
  line.
- `adr answer <id>@<version> refuse` — owner-only. Refuses while
  `adrdom/terms.md` is unapproved; otherwise refuses a question by id,
  writing `**Answer:** refused` and a `refused` ledger row. A refused
  question holds its tier until the owner approves a plan change.
- `adr approve <file>` — owner-only. For a topic file: assign ids to any
  heading that has none, then approve its current plan and/or context (an
  `approved`/`context` ledger row each), refusing first on
  a shape error, a heading with no `depends:` line, a `depends:` entry that
  fails to resolve (no such term, no current decision, no such heading in
  the file, a cycle, a self-dependency), or a tier over the cap. For
  `adrdom/terms.md`: fill in missing term ids and approve it (a `terms` row),
  refusing first on a shape error, a binding that would resolve differently,
  or a newly banned word appearing in a topic file.
- `adr unban` — owner-only. Read lines from stdin and remove them (and
  their tokens) from the ban list.
- `adr ban` — read lines from stdin and ban them (and their tokens) from
  ever appearing in the repository or a commit message.
- `adr show [topic_file]` — print each topic's derived tiers and, under each,
  every question's state (`skeleton`, `unasked`, `open`, `answered <letter>`,
  `refused`, `superseded`, `withdrawn`), what it stands on, and for an
  answered question the chosen sentence.
- `adr serve [--port N]` — owner-only. Serve the owner's page on
  `127.0.0.1` (default port 7357), gated by a per-start token printed to
  stdout; shows status, open questions, decisions, and a `/terms` page
  listing every term, with forms to approve, answer, and unban.
- `adr check [--staged] [--message <path>]` — run every governance check,
  including failing outright while `adrdom/terms.md` differs from its
  latest approval; used by `.githooks/pre-commit` and
  `.githooks/commit-msg`.

`adr answer`, `adr approve`, `adr unban`, `adr serve`, and `adr uninstall`
refuse to run unless invoked interactively by a human (a real TTY on
stdin; no environment variable is consulted) — an AI agent cannot answer a
question, approve anything, edit the ban list, start the server, or
uninstall.

## Topic file shape

A file under `adrdom/decisions/` starts with a first non-blank line `# <topic>`
(exactly one h1), followed by context prose (any text) up to the first
`### ` question heading. A `## ` line is a shape error: tiers are no longer
written by hand, they are derived from `depends:` lines.

Each `### <text>?` question (h3, must end in `?`) opens with exactly one
`depends: <entry>, <entry>, ...` line, comma-separated entries, naming what
the question stands on: a term by name or alias; a heading in the same
file by text, exact match after trim, which may carry an answer set suffix
`=A`, `=A|B`, or `=*` naming the letters of that question's options the
dependent still assumes; or a uuid, which names a heading id in the same
file, a term id, or a current decision's id — and heading text may also
name a current decision's heading text in another topic file. A heading
with no `depends:` line, or more than one, is a shape error. `adr
ask`/`adr approve` derive each question's tier from its heading `depends:`
entries alone (term and decision entries don't affect tier): a question
whose entries name only terms or decisions, never another heading in the
file, is tier 1, a root; otherwise its tier is one more than the highest
tier among the questions it names. A dangling entry, a cycle, or a question
depending on itself refuses. One tier may hold at most 4 live questions (a
value set in the tool for "Does the tool cap the questions in a tier?"; a
different value is a new release); a file that goes over splits the topic
or moves questions under a higher heading.

A question body otherwise holds only: blank lines; `depends: `, `id: `,
`supersedes: `, and `terms: ` lines; one `Problem: ` and one `Standard: `
line; option lines `- X) sentence.` with two-space-indented `  Why: ` and
`  Costs: ` lines under each; and a trailing `**Answer:** <letter>` or
`**Answer:** refused`. Anything else is a shape error. A **skeleton
heading** — one whose body holds only blank lines, `depends:` lines, and an
`id:` line — needs no id of its own and is never a failure, but `adr
approve` gives it one like any other heading. A refused question is
answered for hashing and editing (its text is frozen) but unanswered for
the tier rule and for `done`.

## Terms

`adrdom/terms.md` starts with `# Terms`, followed by `## <name>` terms, each
holding `id:`, `definition:`, `aliases:` (comma-separated, may be empty), an
optional `retired: yes` line, and a `terms:` line approve writes for the words
its definition pairs. Nothing else but blank lines is allowed. A definition
says what the thing is, never what it does, and a circular definition is
refused. `adr ask` resolves a
question's text against every term's name/alias (whole word, case-insensitive,
longest match first) and writes the result under the question's `id: ` line
as `terms: <name>=<uuid>, <name>=<uuid>`.

## Install and check

`adr install --claude` merges a set of `Bash`/`Edit`/`Write` deny entries
into `.claude/settings.json`'s `permissions.deny` (covering `adr answer`,
`adr approve`, `adr unban`, `adr serve`, `adr uninstall`, and edits to
`adrdom/state/`). `adr check`'s `install` check always verifies that
AGENTS.md holds the current brief between its marker lines, and that
`.githooks/pre-commit` and `.githooks/commit-msg` each hold what `adr
install` writes; either failure names `adr install` as the fix (this does
not cover `git config core.hooksPath` itself, which is not a file `check`
can read). The CLAUDE.md-import and deny-entry parts of the
check only run when a `.claude` directory exists in the repository; when it
does, the check fails if CLAUDE.md exists but lacks the import, or any deny
entry is missing, naming `adr install --claude` as the fix. Likewise, the
`.codex/rules/adrdom.rules` part only runs when a `.codex` directory
exists; when it does, the check fails if the file is missing or stale,
naming `adr install --codex` as the fix.

## Ledger

`adrdom/state/ledger.tsv` is append-only, one tab-separated
`time, event, id, hash, detail, version` row per event (`version` is the
tool's `VERSION` that wrote the row; rows from before it carry five fields):
`asked`, `answered`, `refused`, `superseded`, and `withdrawn` track a
question; `assumes` records, at ask time, the answer set a later question
declared on the asked one; `approved` and `context` track a topic file's
plan and context prose; `terms` tracks the terms file; `banned` and
`unbanned` track the ban list.

## Tests

```sh
cargo test --all-features
```

`--all-features` also builds `fake-model`, a hashed bag of words that the test
harness selects with `ADR_FAKE_MODEL=1`, so no test downloads the embedding
model. `ADR_NO_MODEL=1` (with `fake-model`) simulates a machine without it. One
test uses the real model and is skipped unless asked for; it downloads the model
into `adrdom/cache/model` the first time:

```sh
cargo test --all-features --test embed_test -- --ignored
```

`owner-bypass` is a test-only cargo feature (gated further by
`ADR_OWNER_BYPASS=1`) that lets the integration test suite exercise
`answer`/`approve`/`unban` non-interactively. It is not part of the default
feature set and must not be enabled in a release build.
