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

Run `adr show`. It prints every current decision, its heading and under it
the answer the owner picked, and after the decisions of each topic file its
findings and its moves. A finding opens with the rule it breaks and ends
with the move that clears it; a move is one line, opening with who makes
it.

- A line that opens with `agent` is yours. A line with `builds` in it is for
  an agent that did not write the question. Make these moves in the order
  printed. When such a line ends with `owner runs: ...`, make your move,
  then post that command in chat and stop.
- A line that opens with `owner runs:` is the owner's. Post the command in
  chat and stop.
- A line that opens with `not yet` names something whose turn has not come.
  Leave it.

`adr show <file>` takes every file `adr approve` takes: a topic file, the
terms file, a blame file. For a topic file it prints every question, the
lines it carries, and the body of a question that waits for the owner's
answer; what the file holds beyond its latest approval is printed marked,
`+` for a line added and `-` for a line gone, and a question new or struck
as a whole carries the mark on its own line too; then the findings and the
moves of the file. The file itself, a term entry and the prose open with
their status in brackets: `[new]`, `[changed]`, `[approved]`, `[removed]`,
and `[retired]` for a term. `adr show --dependents <file>` also prints what
stands on each question or term entry and what took the place of a question
that was replaced. Each question opens with where it stands, in brackets:

- `[no body]`: a heading of the plan, its body not written.
- `[not asked]`: the body is written, `adr ask` has not run on it.
- `[asked @<version>]`: it waits for the owner's answer; the version is
  what the owner writes after the id in `adr answer`.
- `[changed since asked]`: it was asked and its text was edited since;
  `adr ask` puts it to the owner again.
- `[answered A]`: a current decision, the owner picked option A.
- `[refused]`: the owner ruled that it cannot be settled as asked.
- `[superseded]`: a later decision took its place.
- `[withdrawn]`: it was asked and then struck from the plan.

## Before you build anything

1. Read every current decision in `adr show`. Build a decision when a move
   names it, and not before. Never build a decision whose question you wrote:
   another agent builds it.
2. If no current decision covers what you are about to build, do not build
   it. Write the question (below) and stop until the owner answers.
3. If a current decision looks wrong, say so in chat and stop. Never edit an
   answered question and never work around a decision.

## Asking a question

The plan is made in six steps, each on the record of the one before. Run
`adr plan <file>` at each step: it reads the file, does the step the file is
at, and prints what it read, what it found and the next move.

**1. Write the headings.** A topic file, `adrdom/decisions/<topic>.md`, holds
the questions on one subject: one `# <Topic>` line, a few lines of prose,
then the headings.

```
# Hooks

What the git hooks run and when.

### Does the pre-commit hook run the tests?

### Does check read staged files only?
```

Ask "does X" or "what does X do". A heading that opens with "which",
"where", "how many" or "how much" is refused: it assumes an answer the owner
has not given. One question per heading: a heading with "and" or "or" in it
is refused as two questions.

**2. Run `adr plan <file>`.** For each heading it brings the ten nearest
current decisions and term definitions, and the reader's word on each: same,
depends or unrelated, after one sentence of reason.

**3. Write the lines.** Every neighbour gets a line under its heading, one
entry on one of three lines, comma separated:

- `depends:` the question stands on it: its options make sense only after
  the other is answered. A term by its name, another heading in this file by
  its text, or a current decision by its heading text.
- `supersedes:` the question replaces that current decision.
- `unrelated:` neither.

The reader's word is not a ruling; your line is. Write a `depends:` entry for
every thing the question stands on, a neighbour or not, and none that is not.
Run `adr plan <file>` again: it names every neighbour on no line.

When a question replaces a current decision, write the heading of that
decision on its `supersedes:` line. When a refusal says a heading names more
than one decision, write the id of the one you mean in its place: it is on
the `id:` line under that heading in its topic file.

```
### Does the pre-commit hook run the tests?
depends: hook, Does check read staged files only?
unrelated: Does check print a line that names no move?
```

**4. Walk each question, then write its body.** Write the body of every
heading. Before you write an option, read the code and the decisions it meets
and find out what the option asks for: what the code has to keep, which part
of it has to look and at what moment, and which current decision the option
touches. That is what the option costs: write it on its Costs line. When you
find a second question on the way, add its heading and run `adr plan <file>`.

A question, its prose and its options hold what is asked and nothing about
the day they were written. Three kinds of words are refused:

- what the code does now: "already", "currently", "today", "right now", "at
  the moment", "as built", "as implemented", "as is", "as it is",
  "unchanged", "no change", "keep it", "nothing to build";
- who said it: "said", "told", "agreed", "chose", "decided", "wants",
  "prefers", "in chat", "in the chat", "this session", "as agreed",
  "as discussed", "as decided", and the name of an agent;
- a date.

```
Problem: what breaks, in one line.
Standard: the named practice for this, stated flat.

- A) One sentence that answers the heading, twenty words at most.
  Why: what this gains, twenty words at most.
  Costs: what this loses, twenty words at most.
- B) ...
```

Two or three options. Under a heading that opens with Does, an option holds
one word. Any other option, and every Why and Costs line, holds twenty words
at most; `adr plan` and `adr check` report a longer line, and `adr approve`
and `adr ask` refuse it. Strip each option to its answer to the heading and
compare them: two options that answer it the same way are one option.

A `depends:` entry on another question of this file carries its answer set:
`=A`, `=A|B` or `=*`, the letters of the other question under which this one
still makes sense.

**5. Run `adr plan <file>` again.** For each question with its body it brings
the ten nearest current decisions and term definitions, with the reader's
word: same, consistent, inconsistent, depends or unrelated; and the nearest
other questions of this file, with same, depends or unrelated. Write a line
for each new neighbour. A heading or a body changed after its step is read
again: run `adr plan <file>` until the move it prints is the owner's
`adr approve`.

**6. Ask approval.** Say the plan is ready, post
`owner runs: adr approve <file>` and stop. The owner reads headings, lines
and bodies once. The owner may strike, add or reorder headings before
approving, or write the plan itself and approve it the same way. After
approval every heading has an `id:` line and the entries of its lines are
ids.

**Then run `adr ask <file>`.** It asks the questions and prints one line
for each ledger row it writes: `asked:` for each question, and `assumes:`
for a question that stands on another under an answer set; or it prints in
one run everything that stops it, each line with the move that clears it.
Make every move before you run it again. Commit, post your lean for each
question in chat, post the `owner runs: adr answer ...` lines `adr show`
prints, and stop. The owner puts a letter, or the word `refuse`, in place of
`<letter>`. When the answer is not in a question's answer set, the answer
removes that question and every question standing on it from the file.

**After the answer.** If you wrote the question, stop: another agent builds
its decision from the record alone. Build the decisions the moves name. Tie
each one to a test: put a comment `adr: <id>` inside a test, outside
`adrdom/`, that shows the code follows the decision. Write the test if none
exists, and add it to git. The answers are committed as they are given.
`adr check` reports each untested decision until a test names it; the build
and its tests follow.

Never write or edit `id:`, `terms:` or `**Answer:**` lines yourself.

## Reviewing a build

When you review a build, name the decision behind each behaviour. A behaviour
with no decision behind it is a finding.

## Changing a plan

To add, strike or reword a heading, or change a line or a body: edit the
file, run `adr plan <file>` until the move it prints is the owner's
`adr approve`, say in chat what changed and why, and stop. The owner
approves a plan as a whole, and every change costs them a full re-read: when one heading
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

Yours: `adr show`, `adr plan`, `adr read`, `adr ask`, `adr check`,
`adr install`, `adr ban` (only when the owner says a word is banned) and
`adr ban --rebuild` (only when a refusal names it).

The owner's, refused when you run them: `adr approve`, `adr answer`,
`adr unban`, `adr serve`, `adr uninstall`.
<!-- adrdom brief end -->
