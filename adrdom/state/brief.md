# adrdom

This repository is governed by `adr`. What the code does is a decision: the option the owner picked when they answered a question.
Agents write the questions. Only the owner approves a plan and answers a question, at their own terminal.
The git hooks run `adr check` at every commit and push; check refuses what breaks a decision.

## agents

### Roles

A session that runs `adr plan` is a planner until it ends: it does not build the decisions it planned, and its commits hold only paths under `adrdom/`.
A builder is another session; it builds from the record alone.
A subagent runs under the session's identifier and carries its role. With no session identifier in the environment, no role applies.
The owner's commands are `adr approve`, `adr answer`, `adr unban`, `adr serve` and `adr uninstall`; they refuse an agent. Every other command is the agent's.

### Moves

`adr show` prints every current decision, then the findings and the moves; a clean repository prints decisions only.
A finding is what the tool found and the one move that clears it. A move opens with who makes it:
- `agent`: make it, in the order printed.
- `builder`: another session's; leave it.
- `owner runs: <command>`: post the command in chat and stop.
- `not yet`: its turn has not come; leave it.
A refusal names what failed and the move that clears it; make that move and no other. The owner is not a way past a refusal: never change a file so that the owner's command clears it, and never ask for an approval or an answer the refusal did not name.
Never pass `--no-verify`; never edit or remove the tool's hooks.
The tool writes `adrdom/state/`; its tracked files are staged with the commit, never edited. Approve writes the `id:` and `terms:` lines, answer the `**Answer:**` line.
Behaviour no current decision covers is a question for the owner before it is built. A decision never changes once answered: a later question supersedes it. A decision that looks wrong is said in chat, never edited or worked around.
`adr ban` bans a word the owner does not want in the repository; only the owner unbans.

### Build

Build a decision when a move names it, and not before; never one whose question this session wrote.
A decision's id is on the `id:` line under its heading; `adr show --ids` prints it.
A decision is built once a test outside `adrdom/` carries the comment `adr: <id>` and shows the code follows it; write the test if none exists. One test may carry several `adr:` comments.
`adr check` reports each untested current decision until a test names it. A test whose `adr: <id>` names a decision that is not current is reported and fails nothing: change its `adr:` comment to the decision that took its place, or remove the comment.
Code that follows a decision may carry the comment `adr-code: <id>`, found by search; it counts as no test and no rule judges it.
In a satellite, `adrdom/primary` holds the path to the record; a test there with `adr: <id>` counts for the primary, and a command that writes the record runs in the primary.

### Review

A behaviour is what the code does or prints. In a review of a build, name the decision behind each behaviour the diff adds or changes; a behaviour with no decision behind it is a finding. Findings go in chat.

## decisions

### Terms

Every word the owner gave a fixed meaning is a term, and in the records it means the term only. The terms brief holds one line per term; every agent reads it before it writes a question, code or a reply:

@terms-brief.md

Every question holds at least one term. A new word enters `adrdom/terms.md` under its `# Terms` line:

```
## <name>
definition: What the thing is, never what it does.
aliases: the other forms of the name, comma separated
```

A term's identifier carries its meaning: the name may change, and a former name goes to `aliases:`. A retired term, `retired: yes` in its entry, holds no words. A term is retired only once no question names it: the decisions naming it are superseded first. Terms are approved before any topic file that names them, one entry at a time: `owner runs: adr approve adrdom/terms.md <name>`.

### Topic files

A topic file, `adrdom/decisions/<topic>.md`, holds the questions of one subject: a `# <Topic>` line, a few lines of prose, then the headings.
A heading is a `###` line and one question: "Does X ..." or "What does X ...". One that opens with which, where, how many or how much assumes its answer; one with "and" or "or" is two questions.
Under the heading sit its lines, `depends:`, `supersedes:`, `unrelated:` and `blame:`, entries comma separated; then the body:

```
### Does the pre-commit hook run check?
depends: check, Does check read staged files only?=A
unrelated: Does check print a line with no move on it?

Problem: what breaks, in one line.
Standard: the named practice for this, stated flat.

- A) One sentence that answers the heading, twenty words at most.
  Why: what this gains, twenty words at most.
  Costs: what this loses, twenty words at most.
- B) ...
```

Two or three options. Under a heading that opens with Does, an option is one word with its full stop, `- A) Yes.`; the Why and Costs lines carry the rest. Two options that answer the heading the same way are one option.
A decision binds by its option sentence; Why and Costs explain it, and on a clash the option holds.
A question holds nothing of the day it was written: no word on what the code does now ("already", "currently", "as built", "unchanged"), who said what ("agreed", "decided", "in chat", an agent's name), or a date.

### Plan

`adr plan <file>` does the step the file is at and prints the next move, until the move is `owner runs: adr approve <file>`. The steps:
1. Write the headings, then run plan: for each heading it brings the nearest current decisions and terms, its neighbours.
2. Write a line for every neighbour: `depends:` when the question loses its meaning without it or gains its meaning from it, `supersedes:` when the question replaces that decision, `unrelated:` otherwise. Run plan again until every neighbour is on a line.
3. Write the bodies. Before an option, read the code and the decisions it meets: what the code has to keep is the option's Costs. A second question found on the way is a new heading.
4. Run plan again: it brings the neighbours of each body with the reader's word, `same`, `consistent`, `inconsistent`, `unrelated` or `closes <letter>`, and a reason. The reader's word is not a ruling; the agent's line is. Write a line for each neighbour it brings, and run plan until the move it prints is the owner's.
5. Say the plan is ready, post `owner runs: adr approve <file>` and stop.

A question stands on what it loses its meaning without or gains its meaning from: a term, or a question of the same file; a depends line names at most two questions. Before approval an entry on any line is the term's name or the heading of the question or decision; approve turns them into ids. An entry on an open question carries its answer set, `=A`, `=A|B` or `=*`: the letters under which the question still makes sense.
A question that replaces a current decision names it on `supersedes:`, with every current decision standing on it and those standing on them; each leaves force once the question is answered.
`closes <letter>` says the record leaves only that option of the question standing: the question is decided, strike the heading. Write the neighbour on `supersedes:` instead only when the question is there to change that decision, and name every decision standing on it too. Under `inconsistent`, write it on `supersedes:` or reword the question so the two agree. A line kept against the reader's word is said in chat, with why.
At most four questions, answered or not, stand on one question, and at most four of a file stand on terms alone; a question that stands on a question of its file sits a tier above it. The question these stand on is the one each of them loses its meaning without or gains its meaning from.
The owner reads the whole plan at each approval; a plan changes as a whole, not line by line: when one heading is wrong, check every heading of the file for the same flaw and propose the changes together, once. To change an approved plan, edit the file, run plan to the owner's move, and say in chat what changed and why.

### Ask and answer

`adr approve <file>` asks every unanswered question of the file at once and prints one line per ledger row it writes; or it prints everything that stops it, each with its move. A question carries no lean; the agent's lean goes in chat.
After approve: commit, post the lean for each question, post the `owner runs: adr answer ...` lines `adr show` prints, and stop. The owner answers with a letter or `refuse`.
An answer outside a question's answer set removes that question and every question standing on it from the file. An answer strikes the section of every superseded question; the next commit carries those files.
`adr show <file>` prints each question with its status: `[no body]`, `[not asked]`, `[asked @<version>]`, `[changed since asked]`, `[answered A]`, `[refused]`, `[superseded]`, `[withdrawn]`.

### Blame

An old prose file, the origin, enters the records through a blame file, `adrdom/blame/<name>.md`, which the owner approves before any heading is written from it. Git keeps the text; the blame file copies none of it.

```
origin: docs/adr/old-file.md
blob: <git hash-object docs/adr/old-file.md>

1-9	detail
10-14	decision
15-22	detail
```

A row is a line range, a tab, and one word: `decision` marks a range the owner settles by picking an option, `detail` every other range. A single line is the range `3-3`. The ranges reach every line of the origin, counted as an editor shows them; a range may appear in more than one row. Whether a decision is in the records, or contradicts one, is found in planning, not in the row.
Approve writes an `id:` line under `blob:`. While the origin is in the repository, check compares it with `blob:`: when the origin changes, write the rows and `blob:` again, keep `id:`, and the owner approves again. Once approved, the owner may delete the origin.
Then write the headings from the decision rows, each in the topic file of its subject, with a blame line under the heading: `blame: old-file 10-14 23-30, other 3-9`: the name without `.md`, then the range of each row the question came from; a second blame file after a comma. A row that planning merges into another heading, or drops as a copy, needs no heading; `adr show --questions <blame file>` prints how many rows each question names and how many none does. From there the plan goes its steps.
