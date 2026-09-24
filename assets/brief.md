# adrdom: how an agent works in this repository

This repository is governed by `adr`. Every choice about how the code behaves
is a decision the owner made by answering a question. You write the
questions; only the owner answers them, from their own terminal. The git
hooks run `adr check` on every commit and refuse what breaks a decision.

## Words

- **topic file**: `adrdom/decisions/<topic>.md`, the questions on one subject.
- **plan**: a topic file's `### ` headings and their `depends:` lines.
- **question**: one heading with its body: a problem, a standard, options.
- **decision**: an answered question. It stays current until a later
  decision supersedes it.
- **tier**: questions at the same depth. Tier 1 stands on no other question;
  tier 2 stands on tier 1, and so on. The tool works out the tiers from the
  `depends:` lines; you never write them.

## Before you build anything

1. Run `adr show` and read every current decision. Build under them.
2. If no current decision covers what you are about to build, do not build
   it. Write the question (below) and stop until the owner answers.
3. If a current decision looks wrong, say so in chat and stop. Never edit an
   answered question and never work around a decision.

## Asking a question

**1. Write the plan.** In the topic file: one `# <Topic>` line, a few lines
of prose, then the headings. Under each heading put only `depends:` lines,
one for each thing the question stands on: a term by its name, another
heading in this file by its text, or a current decision by its id.

```
# Hooks

What the git hooks run and when.

### Does the pre-commit hook run the tests?
depends: hook
depends: Does check read staged files only?

### Does check read staged files only?
depends: check
```

Ask "does X" or "what does X do", not "which X", "where" or "how many": those
assume an answer the owner has not given. One question per heading. A tier
holds at most 4 questions. Say the plan is ready and stop. The owner runs
`adr approve <file>`, which gives every heading an `id:` line and replaces
each `depends:` word with the id it names.

**2. Write the bodies of the open tier.** `adr show` prints the tiers; the
lowest one with an unanswered question is open. Under each of its headings:

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

Every `depends:` line elsewhere in the file that points at a question in
this tier also gets an answer set: `=A`, `=A|B` or `=*`, the letters under
which the dependent question still makes sense. `adr show` prints what each
id stands for.

**3. Run `adr ask <file>`.** It writes each question's `terms:` line, or
refuses and says why (a question outside the open tier, a missing answer
set, a question with no term in it, and so on). Commit, post your lean for
each question in chat, and stop. The owner runs `adr answer <id> <letter>`
or `adr answer <id> refuse`.

**4. After the answer.** Tie the decision to a test: put a comment
`adr: <id>` inside a test, outside `adrdom/`, that shows the code follows the
decision. Write the test if none exists. `adr check` lists every decision
still untested.

Never write or edit `id:`, `terms:` or `**Answer:**` lines yourself.

## Changing a plan

To add, strike or reword a heading, or change what it depends on: edit the
file, say in chat what changed and why, and stop. Nothing in the file can be
asked until the owner approves it again. The owner approves a plan as a
whole, and every change costs them a full re-read: when one heading turns
out wrong, check every heading in the file for the same flaw and propose all
the changes together, once. A refused question holds its tier
until the plan changes. Never add a `depends:` line just to move a question
into a later tier.

## Terms

`adrdom/terms.md` fixes the meaning of words; `adr ask` pairs the words of
each question to its terms, and a question with no term in it is refused.
When a question needs a word no term defines, add an entry and stop:

```
## <name>
definition: What the thing is, never what it does.
aliases: other forms, comma separated
```

To stop using a term, add `retired: yes` to its entry; its words are then
free for a new term. Retiring is refused while a current decision or another
term's definition still uses it. The owner runs `adr approve adrdom/terms.md`
after any edit.

## When a commit is refused

Each line of the message names what failed and the command that clears it.
A line ending in `owner runs: ...` is the owner's move: post that command in
chat and stop. The owner is not a way past a refusal: never change a file
just so the owner's command clears it, and never ask for an approval or an
answer the refusal did not name. Never pass `--no-verify`, never edit anything under
`adrdom/state/`, and never remove or edit the hooks.

## Commands

Yours: `adr show`, `adr ask`, `adr check`, `adr install`, and `adr ban` (only
when the owner says a word is banned).

The owner's, refused when you run them: `adr approve`, `adr answer`,
`adr unban`, `adr serve`, `adr uninstall`.
