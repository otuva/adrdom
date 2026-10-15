# adr

## What the tool is and the problems it solves

`adr` is a command-line tool that governs how AI coding agents work in a git repository.

Three problems come up when an agent works in a repository:

- The agent guesses which choices are decisions and which are details.
- The agent writes loaded questions: a heading that assumes its answer, or two questions in one.
- The record of decisions drifts from the code.

What `adr` does about them:

- Decisions are questions in markdown topic files under `adrdom/decisions/`. Agents write the questions. Only the owner approves a plan and answers a question, at their own terminal.
- Every approval and answer is a row in an append-only ledger, `adrdom/state/ledger.tsv`.
- Git hooks run `adr check` at every commit and every push. They refuse a change that breaks a decision.
- A test that carries the comment `adr: <id>` is tied to the decision with that id. `adr check` reports each current decision that no test names.
- Terms in `adrdom/terms.md` fix the meaning of words used in questions.

## Install and uninstall

Build and install the binary from a clone (the package and the binary are both named `adr`):

```sh
cargo install --path .
```

Then run, inside the repository to govern:

```sh
adr install            # the repository setup
adr install --claude   # the setup, plus the Claude Code files
adr install --codex    # the setup, plus the Codex rules file
adr uninstall          # owner-only: remove what install wrote outside adrdom/
```

`adr install` writes these files and one git setting:

- `adrdom/state/ledger.tsv`, `adrdom/state/reader.tsv` and `adrdom/state/banned.sha256`, all empty at first.
- `adrdom/state/brief.md`, the brief an agent reads before it writes a question, code or a reply.
- `adrdom/.gitattributes` and `adrdom/.gitignore`.
- `.githooks/pre-commit` (runs `adr check --staged`), `.githooks/commit-msg` (runs `adr check --message`) and `.githooks/pre-push` (runs `adr check --push`), and `git config core.hooksPath .githooks`.
- `AGENTS.md`, with a short section between `<!-- adrdom brief begin -->` and `<!-- adrdom brief end -->` that imports `adrdom/state/brief.md`.

`adr install --claude` also creates `CLAUDE.md` with an `@AGENTS.md` import when the file is missing, and merges deny entries into `.claude/settings.json`. The entries deny `adr answer`, `adr approve`, `adr unban`, `adr serve` and `adr uninstall`, and edits to `adrdom/state/**`.

`adr install --codex` also writes `.codex/rules/adrdom.rules`, which forbids the owner's commands under Codex's project rules.

`adr uninstall` removes the AGENTS.md section (and the file, if nothing else is in it), the import line it added to `CLAUDE.md`, the three hooks and `core.hooksPath` (only if they are unchanged since install), the deny entries, and `.codex/rules/adrdom.rules`. It leaves `adrdom/` as it is.

## The workflow

```
agent writes terms in adrdom/terms.md
        |
        v
owner: adr approve adrdom/terms.md <name>      (one entry at a time)
        |
        v
agent writes the headings of a topic file
        |
        v
agent: adr plan <file>   (repeat until the move it prints is the owner's approve)
        |
        v
owner: adr approve <file>
        |
        v
agent: adr ask <file>
        |
        v
owner: adr answer <id>@<version> <letter>
        |
        v
another session builds the decision and ties a test to it (an `adr: <id>` comment)
        |
        v
adr check at every commit and push
```

The first session in an empty repository:

1. `adr install` sets up the repository.
2. The agent writes the terms its first questions use into `adrdom/terms.md`.
3. The owner approves them one entry at a time: `adr approve adrdom/terms.md <name>`.
4. The agent writes the first topic file, `adrdom/decisions/<topic>.md`, and runs `adr plan` on it until the next move is the owner's.
5. The owner runs `adr approve` on the topic file. The agent runs `adr ask`. The owner answers.
6. Another session builds each decision and ties a test to it.

The owner's commands. An agent that runs one is refused:

- `adr approve <file> [entry]` approves the headings and prose of a topic file, one entry of `adrdom/terms.md`, or a blame file.
- `adr answer <id>@<version> <letter>` answers a question with an option letter, or with `refuse`.
- `adr serve [--port <port>]` serves the owner's page on 127.0.0.1 (port 7357 by default). The page shows what the terminal prints.
- `adr unban` reads lines from stdin and removes them from the ban list.
- `adr uninstall` removes what `adr install` wrote outside `adrdom/`.

## Record shapes and commands

A question in a topic file (from `adrdom/decisions/terms-brief.md`; its `terms:` line and one option are left out):

```
### Does the tool write a terms brief for the agent?
depends: 7d44030a-3589-4f29-aaa6-12fa09908ab8, f90dbd5b-8fbe-4899-8588-f8762795c2e2
id: ec0b9293-916e-4ced-a3d7-3b862ab7b069

Problem: an agent writes questions, code and chat with term words whose definitions it has never read, because nothing puts them in front of it.
Standard: a shared vocabulary is put in front of every contributor before they write, from one source.

- A) Yes: the tool writes a terms brief from the terms file, a file made for the agent to read.
  Why: the definitions reach the agent as one short file with nothing in it the agent has no use for.
  Costs: one more file, which the tool has to keep equal to the terms file.
- B) No: show prints the name, the aliases and the definition of every term that is not retired.
  Why: no new file, and the brief tells the agent to run show before it builds.
  Costs: the definitions reach the agent only once it runs show, after it has written chat and drafts.

**Answer:** A
```

Output of `adr show adrdom/decisions/terms-brief.md`, first six lines:

```
[approved] Terms brief (adrdom/decisions/terms-brief.md)
[answered A] Does the tool write a terms brief for the agent?
  Yes: the tool writes a terms brief from the terms file, a file made for the agent to read.
[answered A] What does the terms brief hold?
  One line per term that is not retired, with its name, its aliases and its definition.
[answered A] Does approve write the terms brief?
```

Commands:

- `adr install` sets up `adrdom/`, the hooks and `AGENTS.md`; see `adr install --help`.
- `adr uninstall` is owner-only and removes what install wrote outside `adrdom/`; see `adr uninstall --help`.
- `adr ask` asks the owner every question of a topic file that has a body and is not asked yet; see `adr ask --help`.
- `adr plan` brings the neighbours of a topic file's headings and questions, names the lines still to write, and prints the next move; see `adr plan --help`.
- `adr read` reads every current decision against its ten nearest and prints each pair marked same or inconsistent; see `adr read --help`.
- `adr approve` is owner-only and approves a topic file, one terms entry or a blame file; see `adr approve --help`.
- `adr answer` is owner-only and answers a question by `<id>@<version>`; see `adr answer --help`.
- `adr show` prints every current decision and every move, or a file's questions with their status; see `adr show --help`.
- `adr ban` reads lines from stdin and bans them and their tokens from the repository; see `adr ban --help`.
- `adr unban` is owner-only and removes lines from the ban list; see `adr unban --help`.
- `adr serve` is owner-only and serves the owner's page on 127.0.0.1; see `adr serve --help`.
- `adr check` runs all governance checks; see `adr check --help`.
