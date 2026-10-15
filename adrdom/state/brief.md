# adrdom

This repository is governed by `adr`. What the code does is a decision: the option the owner picked when they answered a question.
Agents write the questions. Only the owner approves a plan and answers a question, at their own terminal.
The git hooks run `adr check` at every commit and push; check refuses what breaks a decision.

## agents

`adr show` prints every current decision, then the findings and the moves.
A move opens with who makes it: `agent`, `builder` (another session), `owner runs: <command>` (the owner, at their terminal), or `not yet`.
A finding is what the tool found and the one move that clears it; an owner command clears nothing a finding did not name.
Behaviour no current decision covers is a question for the owner before it is built; a decision never changes once answered, a later question supersedes it.
A session that runs `adr plan` or `adr ask` is a planner until it ends: it does not build the decisions it asked, and its commits hold only paths under `adrdom/`. A subagent runs under the session's identifier and carries its role.
A builder is another session; it builds from the record alone, and a decision is built once a test outside `adrdom/` carries the comment `adr: <id>` and shows the code follows it.
In a review of a build, a behaviour with no decision behind it is a finding.
The tool writes `adrdom/state/`; its tracked files are staged with the commit, never edited. Approve writes the `id:` lines, ask the `terms:` line, answer the `**Answer:**` line.
`adr ban` bans a word the owner does not want in the repository; only the owner unbans.

## decisions

Every word the owner gave a fixed meaning is a term, and in the records it means the term only. The terms brief holds one line per term; every agent reads it before it writes a question, code or a reply:

@terms-brief.md

Every question holds at least one term. A new word enters `adrdom/terms.md`, as `## <name>`, a `definition:` line and an `aliases:` line; a definition says what the thing is, never what it does. Terms are approved before any topic file that names them.
A topic file, `adrdom/decisions/<topic>.md`, holds the questions of one subject: a `# <Topic>` line, a few lines of prose, then the headings.
A heading is one question: "Does X ..." or "What does X ...". One that opens with which, where, how many or how much assumes its answer; one with "and" or "or" is two questions.
The body:

```
Problem: what breaks, in one line.
Standard: the named practice for this, stated flat.

- A) One sentence that answers the heading, twenty words at most.
  Why: what this gains, twenty words at most.
  Costs: what this loses, twenty words at most.
- B) ...
```

Under a heading that opens with Does, an option is one word. Two options that answer the heading the same way are one option.
A question holds nothing that ties it to the day it was written: no word on what the code does now, who said what, or a date.
Under each heading, every neighbour `adr plan` brings goes on one line: `depends:`, `supersedes:` or `unrelated:`.
A question stands on what it loses its meaning without: a term, or a question of the same file; a depends line names at most two questions. An entry on an open question carries its answer set, `=A`, `=A|B` or `=*`: the letters under which the question still makes sense.
A question that replaces a current decision names it on `supersedes:`, with every current decision standing on it; each leaves force once the question is answered.
A tier holds at most four questions; tiers are unlimited: a question that stands on a question of its file sits a tier above it. The question a tier stands on is the one each of its questions loses its meaning without.
`adr plan <file>` does the step the file is at and prints the next move, until the move is `owner runs: adr approve <file>`.
The owner reads the whole plan at each approval, so a plan changes as a whole, not line by line.
`adr ask <file>` asks every unanswered question of the file at once; a question carries no lean, the agent's lean goes in chat.
An old prose file enters the records through a blame file under `adrdom/blame/`, which the owner approves before any heading is written from it.
