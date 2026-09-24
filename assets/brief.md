# adrdom: how an agent works in this repository

This repository is governed by `adr`. Decisions live in topic files under
`adrdom/decisions/`, one file per topic. Only the owner answers or refuses a
question, from their own terminal. You draft questions; you never answer or
refuse them. The git hooks run `adr check` on every commit and refuse what
breaks a decision.

## Before you build anything

1. Run `adr show` and read every current decision. Build under them.
2. If a current decision does not cover what you are about to build, do not
   build it. Write the question (below) and stop until the owner answers.
3. If a current decision looks wrong, say so in chat and stop. Never edit an
   answered question and never work around a decision.

## How a decision gets made

1. Headings first. Open or create `adrdom/decisions/<topic>.md`: one
   `# <Topic>` line, a few lines of prose, then `### <question>?` headings
   with nothing under them but a `depends:` line for each thing the question
   stands on — a term by name, a heading in this same file by its text, or
   a current decision by its id. Never write a `## ` tier heading; tiers are
   derived from the depends lines, not written by hand. Prefer a heading
   that asks "does X" or "what does X do" over "which X" / "where" / "how
   many": a loaded heading presupposes an answer the owner hasn't given.
   Say the file is ready and stop. The owner runs one
   `adr approve adrdom/decisions/<topic>.md` for the whole plan, which turns
   each depends line into the id it names and gives every heading, skeleton
   included, its own `id:` line.
2. Bodies next, one tier at a time. `adr show` prints the tiers `adr ask`
   derives from the depends lines; only the lowest tier with an unanswered
   question is open. Under each heading in the open tier write one
   `Problem:` line (what breaks), one `Standard:` line (the named practice,
   stated flat), and two or three options as `- A) One sentence.` each with
   an indented `Why:` and `Costs:` line. While writing these bodies, also
   put an answer set on every `depends:` line elsewhere in the file that
   points into this tier — `=A`, `=A|B`, or `=*` — naming the letters under
   which the dependent question still stands. Before running `ask`, strip
   each option down to its answer to the heading and compare the options
   against each other; two options that answer the heading the same way are
   one option, not two.
3. Run `adr ask adrdom/decisions/<topic>.md`. It refuses a question outside
   the open tier, a tier over the cap, a question standing on a refused one,
   a question left moot by an answer already given, a tier where some
   depends line into it still has no answer set, or a heading with no id at
   all (the plan isn't approved yet — `adr approve` first). On success it
   writes each new question's `terms:` line. Never write or edit `id:`,
   `terms:`, or `**Answer:**` lines yourself; an answer set (`=A`, `=A|B`,
   `=*`) goes on the depends line once it already names an id — `adr show`
   prints the word or heading each id stands for, so you never have to
   read the id itself. Commit, post your lean per question in chat, and
   stop. The owner runs `adr answer <id> <letter>` or `adr answer <id>
   refuse`.
4. A refusal holds the tier: nothing that stands on it can be asked until
   the plan changes. Propose the change in chat — drop the question, reword
   it, or move what stands on it — and stop. The owner approves it again.
5. Once answered, tie the decision to a test: a comment `adr: <id>` inside
   the test that shows the code follows it, outside `adrdom/`. Write the
   test if none exists. `adr check` names every decision still untested.

## Changing a plan

The approved headings and their depends lines are the plan. To add, strike,
reword a heading, or change what it depends on, edit the file, say in chat
what changed and why, and stop. Nothing in that file can be asked until the
owner approves it again. Never add a depends line just to push a question
into a later tier to get past it.

## Terms

`adrdom/terms.md` fixes the meaning of words. `adr ask` pairs every word in a
question's own text to a term by name or alias; a `depends:` line names its
term by id once `adr approve` has resolved it, not by the word you wrote.
When a question needs a word no term defines, add a `## <name>` entry with a
`definition:` line saying what the thing is (never what it does) and an
`aliases:` line, then say so and stop. The owner runs
`adr approve adrdom/terms.md`.

## When a commit is refused

Read the message. Each line names what failed and the command that clears
it; a line ending in `owner runs: ...` is the owner's move, so post that
command in chat and stop. Never pass `--no-verify`, never edit anything
under `adrdom/state/`, and never remove or edit the hooks.

## Commands

Yours: `adr show`, `adr ask`, `adr check`, `adr install`, and `adr ban`
(only when the owner says a word is banned).

The owner's, refused when you run them: `adr answer <id> <letter>`,
`adr answer <id> refuse`, `adr approve`, `adr unban`, `adr serve`,
`adr uninstall`.
