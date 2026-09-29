<!-- adrdom brief begin -->
# adrdom: how an agent works in this repository

This repository is governed by `adr`. Every choice about how the code behaves
is a decision the owner made by answering a question. You write the
questions; only the owner answers them, from their own terminal. The git
hooks run `adr check` on every commit and refuse what breaks a decision.

## Words

Every word the owner has given a fixed meaning is in the terms brief, one
line for each: its name, its other forms in brackets, its definition. Read
that file before you write a question, code or a reply. The next line is its
path, and some agents load the file from it at the start of a session:

@adrdom/state/terms-brief.md

## What is next

Run `adr show`. It prints every question with its state, every current
decision, and under each topic file the lines of `next`: one line for each
move, opening with who makes it.

- A line that opens with `agent` is yours. Make these moves in the order
  printed.
- A line that opens with `owner runs:` is the owner's. Post the command in
  chat and stop.
- A line that opens with `not yet` names something whose turn has not come.
  Leave it.

## Before you build anything

1. Read every current decision in `adr show`. Build a decision when `next`
   names it, and not before.
2. If no current decision covers what you are about to build, do not build
   it. Write the question (below) and stop until the owner answers.
3. If a current decision looks wrong, say so in chat and stop. Never edit an
   answered question and never work around a decision.

## Asking a question

**1. Write the plan.** A topic file, `adrdom/decisions/<topic>.md`, holds the
questions on one subject: one `# <Topic>` line, a few lines of prose, then
the headings. Under each heading put one `depends:` line, comma separated:
one entry for each thing the question stands on -- a term by its name,
another heading in this file by its text, or a current decision by its
heading text, the same as a heading in this file.

```
# Hooks

What the git hooks run and when.

### Does the pre-commit hook run the tests?
depends: hook, Does check read staged files only?

### Does check read staged files only?
depends: check
```

A question stands on another one when its options make sense only after the
other is answered. Write every entry that is so, and none that is not.

Ask "does X" or "what does X do", not "which X", "where" or "how many": those
assume an answer the owner has not given. One question per heading. Say the
plan is ready and stop. The owner may strike, add or reorder headings before
approving, or write the plan itself and approve it the same way. The owner
runs `adr approve <file>`; after it every heading has an `id:` line and the
entries of its `depends:` line are ids.

**2. Walk each question, then write its body.** Write the bodies `next`
names. Before you write an option, read the code and the decisions it meets
and find out three things: what the tool has to keep or do for this option
that it does not today, which command looks and at what moment, and which
current decision the option touches. What you find is what the option costs:
write it on its Costs line. When you find a second question on the way, add
its heading to the plan (see Changing a plan).

```
Problem: what breaks, in one line.
Standard: the named practice for this, stated flat.

- A) One sentence that answers the heading.
  Why: what this gains.
  Costs: what this loses.
- B) ...
```

Two or three options. Strip each option to its answer to the heading and
compare them: two options that answer it the same way are one option. If the
question replaces a current decision, add `supersedes: <id>` under the
heading (several ids separated by commas).

When `next` names an answer set, write it on that `depends:` entry: `=A`,
`=A|B` or `=*`, the letters of the other question under which this one still
makes sense.

**3. Run `adr ask <file>`.** It asks the questions, or says what stops it
and the move that clears it. Commit, post your lean for each question in
chat, post the `owner runs: adr answer ...` lines that `next` prints, and
stop. The owner puts a letter, or the word `refuse`, in place of `<letter>`.

**4. After the answer.** Build the decisions `next` names. Tie each one to a
test: put a comment `adr: <id>` inside a test, outside `adrdom/`, that shows
the code follows the decision. Write the test if none exists, and add it to
git. Commit the answers together with their code and tests.

Never write or edit `id:`, `terms:` or `**Answer:**` lines yourself.

## Changing a plan

To add, strike or reword a heading, or change what it depends on: edit the
file, say in chat what changed and why, and stop. The owner approves a plan
as a whole, and every change costs them a full re-read: when one heading
turns out wrong, check every heading in the file for the same flaw and
propose all the changes together, once.

## Terms

`adrdom/terms.md` fixes the meaning of words. Every question uses at least
one term. Terms are approved before any topic file that names them. When a
question needs a word no term defines, add an entry and stop:

```
## <name>
definition: What the thing is, never what it does.
aliases: other forms, comma separated
```

The file starts with a `# Terms` line. To stop using a term, add
`retired: yes` to its entry. The owner runs `adr approve adrdom/terms.md`
after any edit.

## Blame

An old prose file (an origin) enters the records through a blame file,
`adrdom/blame/<name>.md`, which maps every line of the origin to where it
went. Git keeps the text; the blame file is the proof that nothing was
dropped in silence. Write it after the plan is approved, so the headings
exist:

```
origin: docs/adr/old-file.md
blob: <git hash-object docs/adr/old-file.md>

1-9	dropped: dead
10-14	prose: <topic file name>
15-22	decision: <heading of a question in a topic file>
23-23	term: <name>
24-30	dropped: copy of <heading of a question in a topic file>
31-35	dropped: copy of docs/adr/other-file.md
36-40	dropped: detail
```

A row is a line range, a tab, and one destination. Lines are counted as an
editor shows them, so a last line with no newline still counts (`wc -l`
misses it). The ranges must reach every line of the origin; a range may
appear in more than one row. The destinations: `decision:` for text that
lives in a question's section (its answer, its Why or its Costs), `term:`,
`prose:` for the text above a topic file's first question, and `dropped:`
with one reason: `dead` (contradicted or obsolete), `detail` (the code is
the record), or `copy of` what says the same thing elsewhere: a question by
its heading (any state), or another origin by its path. Nothing else is
accepted. The owner runs `adr approve adrdom/blame/<name>.md`. Verification
passes by other agents edit the rows; the owner approves once at the end.
When an origin is renamed, update the `origin:` line. Once every row is
closed, delete the origin.

## When a command refuses

Each line of the message names what failed and the move that clears it. Make
the move it names and no other. A line ending in `owner runs: ...` is the
owner's move: post that command in chat and stop. The owner is not a way
past a refusal: never change a file just so the owner's command clears it,
and never ask for an approval or an answer the refusal did not name. Never
pass `--no-verify`, never edit anything under `adrdom/state/`, and never
remove or edit the hooks.

## Commands

Yours: `adr show`, `adr ask`, `adr check`, `adr install`, and `adr ban` (only
when the owner says a word is banned).

The owner's, refused when you run them: `adr approve`, `adr answer`,
`adr unban`, `adr serve`, `adr uninstall`.
<!-- adrdom brief end -->
