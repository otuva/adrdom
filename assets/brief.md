# adrdom: how an agent works in this repository

This repository is governed by `adr`. Decisions live in topic files under
`adrdom/decisions/`, one file per topic. Only the owner answers questions,
from their own terminal. You draft questions; you never answer them. The
git hooks run `adr check` on every commit and refuse what breaks a decision.

## Before you build anything

1. Run `adr show` and read every current decision. Build under them.
2. If a current decision does not cover what you are about to build, do not
   build it. Write the question (below) and stop until the owner answers.
3. If a current decision looks wrong, say so in chat and stop. Never edit an
   answered question and never work around a decision.

## How a decision gets made

1. Headings first. Create or open `adrdom/decisions/<topic>.md`: one
   `# <Topic>` line, a few lines of prose, then `## <tier>` headings holding
   `### <question>?` headings with nothing under them. Tiers are asked in
   order; a later tier waits for every answer in the earlier one. Say the
   file is ready and stop. The owner runs `adr approve adrdom/decisions/<topic>.md`.
2. Bodies next, one tier at a time. Under each approved heading write one
   `Problem:` line (what breaks), one `Standard:` line (the named practice,
   stated flat), and two or three options as `- A) One sentence.` each with
   an indented `  Why:` and `  Costs:` line. Nothing else goes under a heading.
3. Run `adr ask adrdom/decisions/<topic>.md`. It gives each question an
   `id:` line and a `terms:` line. Never write or edit `id:`, `terms:`, or
   `**Answer:**` lines yourself. Commit, post your lean per question in
   chat, and stop. The owner runs `adr answer <id> <letter>`.
4. Once answered, tie the decision to a test: a comment `adr: <id>` inside
   the test that shows the code follows it, outside `adrdom/`. Write the
   test if none exists. `adr check` names every decision still untested.

## Changing a plan

The approved headings are the plan. To add, strike, or move a heading, edit
the file, say in chat what changed and why, and stop. Nothing in that file
can be asked until the owner approves it again. Never move a question to a
later tier to get past it.

## Terms

`adrdom/terms.md` fixes the meaning of words. `adr ask` pairs every word in a
question to a term by name or alias. When a question needs a word no term
defines, add a `## <name>` entry with a `definition:` line saying what the
thing is (never what it does) and an `aliases:` line, then say so and stop.
The owner runs `adr approve adrdom/terms.md`.

## When a commit is refused

Read the message. Each line names what failed and the command that clears
it; a line ending in `owner runs: ...` is the owner's move, so post that
command in chat and stop. Never pass `--no-verify`, never edit anything
under `adrdom/state/`, and never remove or edit the hooks.

## Commands

Yours: `adr show`, `adr ask`, `adr check`, `adr install`, and `adr ban`
(only when the owner says a word is banned).

The owner's, refused when you run them: `adr answer`, `adr approve`,
`adr unban`, `adr serve`, `adr uninstall`.
