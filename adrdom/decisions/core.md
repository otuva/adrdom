# Core

What the tool holds fixed in every repository: what an answered question may no longer do, who
may run the owner's commands, what a question stands on, and the order questions reach the owner.

### Does a decision change after it is answered?
depends: 41c41584-1ea3-4f15-b7ec-76a1dc99d32f, b36113a1-1384-4e4c-a798-8399f4181366, 2bae531b-c4df-48a0-89a0-3d52726741ea
id: dad5273c-afe2-4ab1-a451-e2a5d4d9a26d
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answered=b36113a1-1384-4e4c-a798-8399f4181366, answers=b36113a1-1384-4e4c-a798-8399f4181366, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, identifier=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, supersedes=2bae531b-c4df-48a0-89a0-3d52726741ea

Problem: when an answered decision can be edited, the spec moves under the owner and nobody can say what the owner picked when a line of code was written.
Standard: architecture decision records are immutable once accepted; a change is a new record that supersedes the old one.

- A) No: an answered decision never changes; it leaves force only when a newer decision supersedes it.
  Why: what the owner picked stays readable exactly as picked, and every change is a new question the owner answers.
  Costs: a decision that is slightly wrong is replaced whole by a new question, and its test moves to the successor.
- B) Yes, through separate owner commands to amend, drop or split a decision, each written to the ledger.
  Why: each kind of change has its own named route.
  Costs: three more owner commands to guard, and a decision's sentence changes under its own identifier.
- C) Yes: the owner edits the decision in place and the ledger records the edit.
  Why: the cheapest change for the owner.
  Costs: the text an agent built against moves without a new question, which is how a spec drifts.

**Answer:** A

### Does the tool guard against an agent that works around it on purpose?
depends: 1d7e59ea-99e3-4d4b-9928-5dabca62723d, f90dbd5b-8fbe-4899-8588-f8762795c2e2
id: 98d5d389-8315-41da-9970-666e31f3a94d
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, installed=1b98cdea-dc1b-4789-8f67-68d130d7a49d, installs=1b98cdea-dc1b-4789-8f67-68d130d7a49d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d

Problem: a gate looks strong or weak depending on the attacker assumed, and with no attacker named every owner-only mechanism is argued without a yardstick.
Standard: threat modelling names the adversary before choosing a control; controls against a careless insider and a hostile one are different systems.

- A) No: the tool guards against an agent that runs an owner command because it looks helpful in the moment, not one that works to get past the tool.
  Why: this is the failure every recorded case shows, and a guard against it fits inside one installed tool.
  Costs: an agent that sets out to pass as the owner gets through.
- B) Yes: the tool guards against an agent that actively works to get past it.
  Why: covers every agent.
  Costs: only a separate account or machine for the owner holds against that, which is a setup the owner builds, not a tool the owner installs.

**Answer:** A

### Does ask stop a question outside the plan?
depends: 3c8d4c9c-abca-4faa-b7c1-aae39951376c, 94677ed8-eaf6-46e2-bc71-7392fa0805bf, e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd
id: da59b448-1afb-4469-b40e-69eda8f253c2
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approved=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, asked=3c8d4c9c-abca-4faa-b7c1-aae39951376c, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: an agent with a topic file open writes every question it can think of in one pass, and the owner faces a pile of questions instead of a design.
Standard: spec-driven tools have the human approve a plan before tasks are written; most keep approval as a file state and do not stop work that ignores it.

- A) Yes: ask stops any question whose heading is not in the plan the owner approved.
  Why: a rule written only as text did not hold within a single session; a stop in the tool does.
  Costs: every topic file costs the owner an approve before any question is asked.
- B) No: the agent shows the plan to the owner in conversation, and nothing in the tool reads it.
  Why: nothing new in the tool.
  Costs: the agent is the only guard, and the agent is the party that writes the pile.
- C) No: the tool records the approved plan and check warns about questions outside it, stopping nothing.
  Why: the plan is on record without a wall.
  Costs: a warning an agent can walk past holds no better than B.

**Answer:** A

### Does check fail a current decision with no test?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, e2533580-1486-4dc9-8697-e6c53605fe72, 41c41584-1ea3-4f15-b7ec-76a1dc99d32f
id: 33a45b9e-8150-488e-be70-7ab2ca9adaa6
terms: answer=b36113a1-1384-4e4c-a798-8399f4181366, answers=b36113a1-1384-4e4c-a798-8399f4181366, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, checks=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, current=e2533580-1486-4dc9-8697-e6c53605fe72, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, decisions=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d

Problem: a decision with nothing that checks it does not hold; the code drifts from it and nothing reports the drift.
Standard: requirements traceability links every requirement to a test that verifies it and reports a gap as a failure.

- A) Yes: check fails while any current decision has no test tied to it.
  Why: every decision the owner answers is held by code that fails when the decision is broken.
  Costs: an answer cannot be committed until its test exists.
- B) No: check reports untested decisions as a count and fails nothing.
  Why: an answer lands before its code.
  Costs: the count grows and nothing stops it.

**Answer:** A

### Does check fail an answered question whose text changed?
depends: dad5273c-afe2-4ab1-a451-e2a5d4d9a26d=A, 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, e18b82d8-5271-4385-bbab-2b89e5da278f
id: cd83e277-ebf0-4458-a77a-c526ab4cfeca
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answered=b36113a1-1384-4e4c-a798-8399f4181366, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, depends=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f, question=277a8338-c57a-43cc-8b28-6b42e9347f6a

Problem: an answered decision never changes, but it sits in a text file an agent can edit; an edit nothing notices moves the decision silently.
Standard: tamper-evident logs store a hash of each record and verify it on every read.

- A) Yes: the ledger holds a hash of each answered question's text, and check fails any answered question whose text no longer matches it.
  Why: a silent edit becomes a failed check at the next commit.
  Costs: a harmless edit, such as a typo fix, fails the same way and needs a new question.
- B) No: git history shows every edit, and review catches it.
  Why: nothing new in the tool.
  Costs: the catch depends on someone reading the diff.

**Answer:** A

### What does an owner command look for before it runs?
depends: 98d5d389-8315-41da-9970-666e31f3a94d=*, 1d7e59ea-99e3-4d4b-9928-5dabca62723d, f90dbd5b-8fbe-4899-8588-f8762795c2e2
id: aa275fac-def5-42a2-8a61-660142acad8e
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answer=b36113a1-1384-4e4c-a798-8399f4181366, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d

Problem: an agent that runs an owner command because it seems helpful turns its own proposal into the owner's act; the command has to tell the two apart without a login.
Standard: allowlist over denylist; recognise the known principal instead of listing the threats.

- A) A sign of a person at the keyboard, an interactive terminal; without it the command stops, whatever the agent.
  Why: a new agent needs no update to the tool, and every agent stops by the same rule.
  Costs: an agent that opens a terminal on purpose passes, which the threat the tool guards against leaves out.
- B) Traces that known agent programs leave, such as the environment variables they set; the command stops when it finds one.
  Why: a recognised agent stops every time.
  Costs: an agent not on the list counts as the owner, and the list goes stale with every new agent.
- C) A secret the owner types on every owner command, such as a password.
  Why: holds even when an agent opens a terminal.
  Costs: the owner types it on every approve and answer, against a threat the tool does not guard against.

**Answer:** A

### What does a depends line name?
depends: da59b448-1afb-4469-b40e-69eda8f253c2=A|C, aef1b384-2b33-47a2-92a8-ec7f8fb258ef
id: ad9761e2-6d64-4b45-b0ba-5c5d8d2b38bf
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answer=b36113a1-1384-4e4c-a798-8399f4181366, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, checks=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, current=e2533580-1486-4dc9-8697-e6c53605fe72, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, depends=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, options=43778946-d44c-4acf-a766-0a4915fce199, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, stands on=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, term=f590059d-b681-40ec-8a1c-c8916d71f6bb, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: a question that assumes another question's answer, or a behaviour no decision covers, carries that assumption nowhere the tool or the owner can see; it surfaces only when the owner reads the options.
Standard: dependency-ordered task lists carry a prerequisite field per item and stop an item whose prerequisite does not resolve.

- A) What the question stands on: a term, a question in the same topic file, or a current decision; every question carries the line, and approve and ask stop a line that is missing or names anything that does not resolve.
  Why: the assumption sits where the tool checks it and the owner reads it beside the heading; a question that needs a behaviour no decision covers has nothing to name, which shows the missing higher question.
  Costs: the agent still chooses what to write, and an assumption it leaves out passes.
- B) The same three kinds of target, with the line optional: a question with no line stands on nothing, and a line that names something unresolved is stopped.
  Why: a question that stands on nothing costs no line.
  Costs: a missing line and a forgotten assumption look the same.
- C) Free text the owner reads and the tool never resolves.
  Why: the agent states the assumption in its own words.
  Costs: the tool checks nothing, and no order can be derived from it.

**Answer:** A

### What ties a test to a decision?
depends: 33a45b9e-8150-488e-be70-7ab2ca9adaa6=*, 41c41584-1ea3-4f15-b7ec-76a1dc99d32f
id: 0b962bc8-05a8-48da-9fe8-bf77c7aa6e4a
terms: decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, identifier=e877e4a9-2caf-4fe6-9f82-5476a2abb00b

Problem: the tool runs in repositories of any language and has to find which test holds which decision without parsing that language.
Standard: cross-language tools mark a file with a comment carrying an identifier in a fixed form and find it by text search.

- A) A comment `adr: <id>` inside the test, found by text search in every tracked file outside adrdom.
  Why: works in any language, one rule, and people can search for it too.
  Costs: a comment can sit beside a test that asserts nothing; the tie names the test, not its honesty.
- B) A test file named by the decision's identifier, under a directory the tool owns.
  Why: the file being there is the tie.
  Costs: the repository's tests move into the tool's layout.
- C) A line in the decision naming the test by path and name.
  Why: one place to look.
  Costs: a test rename breaks a decision that never changes.

**Answer:** A

### Does the tool derive tiers from the depends lines?
depends: ad9761e2-6d64-4b45-b0ba-5c5d8d2b38bf=A|B, f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c
id: aec860fd-22ad-44a7-99f7-222ea19537e7
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answers=b36113a1-1384-4e4c-a798-8399f4181366, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, asks=3c8d4c9c-abca-4faa-b7c1-aae39951376c, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, depends=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, stands on=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, tier=f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c, tiers=f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: when the agent writes the tiers and fills them, it fills them by subject, and questions that assume each other's answers sit side by side and reach the owner together.
Standard: build systems derive execution order from declared dependencies and stop on a cycle; nobody writes the order by hand.

- A) Yes: the tool derives the tiers from the depends lines and nothing in the topic file states them; a question standing on no other question in its file is in the first tier, any other one tier above the highest question it stands on; approve stops a cycle, ask asks the lowest tier with an unanswered question, and show and the page print the tiers.
  Why: the agent never places a question, so a question cannot sit beside one it stands on.
  Costs: the raw file does not show the order without show or the page.
- B) No: the agent writes the tier headings, and check fails a file where a question sits in the same tier as a question it stands on, or below it.
  Why: the raw file shows the order.
  Costs: two sources for one order, and every mismatch is a failure the agent fixes by hand.
- C) No: the agent writes the tier headings, and nothing compares them with the depends lines.
  Why: nothing new in the tool.
  Costs: the depends line orders nothing.

**Answer:** A

### Does a reference in a topic file carry the identifier of what it names?
depends: ad9761e2-6d64-4b45-b0ba-5c5d8d2b38bf=A|B, e877e4a9-2caf-4fe6-9f82-5476a2abb00b, 08193e75-dd89-4857-8e9f-b19e1a9e75e6
id: 7cad9922-9bd1-4c1a-aedc-df1f77c83e1f
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, decisions=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, identifier=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, term=f590059d-b681-40ec-8a1c-c8916d71f6bb, terms=f590059d-b681-40ec-8a1c-c8916d71f6bb, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: when a heading is reworded or a term renamed, every reference that names it by its text breaks, and each one is fixed by hand.
Standard: reference by a stable identifier; the reference points at a value that never changes, and the name is only shown to people.

- A) Yes: every reference carries the identifier of what it names; the agent writes the name, approve turns it into the identifier, and show and the page turn it back into the name.
  Why: a rename breaks no reference, and terms, decisions and questions follow one rule.
  Costs: the raw file does not say what a reference points at without show or the page.
- B) No: a reference carries the text of what it names; a rename breaks it and the agent fixes it by hand.
  Why: the raw file reads on its own.
  Costs: every rename is an edit on every line that points at the target, and a second rule beside the identifier rule for the same job.

**Answer:** A

### Does the tool cap the questions in a tier?
depends: aec860fd-22ad-44a7-99f7-222ea19537e7=*, f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c
id: 15d82e53-e344-4347-b639-3bf6eb9ef1b4
terms: answer=b36113a1-1384-4e4c-a798-8399f4181366, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, terms=f590059d-b681-40ec-8a1c-c8916d71f6bb, tier=f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c, topic=292a10b8-c287-4d71-b2bd-840da0a6de61

Problem: nothing bounds a tier; a large tier puts many questions before the owner at once, and a bad question found late in it costs every answer around it its context.
Standard: kanban caps work in progress at every column and stops a pull past the cap.

- A) Yes: four questions per tier, a value set in the tool; approve and ask stop a plan with a tier over four, and a different value is a new question and a new release of the tool.
  Why: the harm of one bad question is bounded, and a large plan passes only by being deep.
  Costs: a topic with more than four questions that stand on nothing but terms needs a higher question or a split.
- B) Yes: four per tier unless the owner sets another value with an owner command the ledger records.
  Why: the value moves without a question or a release.
  Costs: one more owner command and one more ledger event to guard, for a value no plan has pushed against.
- C) No: the owner declines to answer a tier that is too large.
  Why: nothing new in the tool.
  Costs: the pile of questions returns as one large tier.

**Answer:** A

### Does a depends entry carry the answers it holds under?
depends: aec860fd-22ad-44a7-99f7-222ea19537e7=*, 9d634996-475e-4ddc-8113-05c404ad9d5c
id: fa90692a-150e-4a89-91e2-1d575cb59d0f
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answer=b36113a1-1384-4e4c-a798-8399f4181366, answered=b36113a1-1384-4e4c-a798-8399f4181366, answers=b36113a1-1384-4e4c-a798-8399f4181366, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, depends=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, refusal=0942e445-0163-423e-8ba7-84dd8ac07045, refuses=0942e445-0163-423e-8ba7-84dd8ac07045, tier=f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c

Problem: an answer in one tier can make a question in the next tier moot or false; a moot question that reaches the owner costs a refusal, or, answered, plants a decision inconsistent with the one that made it moot.
Standard: a dependency graph records which outcome each node needs and drops the nodes whose outcome did not happen.

- A) Yes: when a tier's bodies are written, every depends entry naming one of its questions gets the letters it holds under (=A, =A|B or =*); ask stops the tier until every such set is written and freezes the sets with the bodies; once the tier is answered, a question whose set lacks the answer is moot, ask stops it, check reports it, and it leaves at the next plan change.
  Why: the assumption is declared before the answer exists and frozen with the question the owner reads, so a moot question never reaches the owner.
  Costs: one more thing the agent writes per dependent question, and a wrong set drops a live question until the owner sees it named at the plan change.
- B) No: the agent re-reads the remaining headings when it writes the next tier's bodies and proposes changes in conversation.
  Why: nothing new in the tool.
  Costs: a moot question the agent misses reaches the owner.
- C) No: the owner refuses a moot question when it arrives.
  Why: nothing for the agent to remember.
  Costs: every moot question costs the owner a move.

**Answer:** A

### Does check at a commit read the staged files?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, cd83e277-ebf0-4458-a77a-c526ab4cfeca
id: 34e81f57-9b3c-4dc4-a2ba-ca227523177d
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answered=b36113a1-1384-4e4c-a798-8399f4181366, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, question=277a8338-c57a-43cc-8b28-6b42e9347f6a

Problem: a changed answered question can be staged while the working tree holds the old text, and check reads the working tree, so the commit passes.
Standard: pre-commit hooks judge the content the commit records, the way the pre-commit framework and lint-staged set unstaged changes aside first.

- A) Yes: at a commit, check reads every file it judges from the staged content, and the working tree only when it runs outside a commit.
  Why: what check passes is exactly what enters git.
  Costs: a fix left unstaged does not count until it is staged.
- B) No: check reads the working tree at a commit as it does outside one.
  Why: one way of reading, and the result matches what the agent sees on disk.
  Costs: a staged file that differs from the working tree enters git unchecked.

**Answer:** A

### Does check fail a test tied to a decision that is not current?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, e2533580-1486-4dc9-8697-e6c53605fe72, 0b962bc8-05a8-48da-9fe8-bf77c7aa6e4a
id: b871e776-2da5-4f1e-b4c4-fb8229657e25
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, current=e2533580-1486-4dc9-8697-e6c53605fe72, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, identifier=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, refused=0942e445-0163-423e-8ba7-84dd8ac07045, supersede=2bae531b-c4df-48a0-89a0-3d52726741ea, superseded=2bae531b-c4df-48a0-89a0-3d52726741ea

Problem: a test comment that names a superseded or refused decision points readers at a rule no longer in force.
Standard: traceability links are kept current, so every link from a test names a requirement still in force.

- A) Yes: check fails every test comment whose identifier is not a current decision, and the agent removes it or points it at the current one.
  Why: every tie names a decision in force, so a search for an identifier finds only the tests that show it.
  Costs: each supersede means editing the old ties before the next commit passes.
- B) No: check reports a tie to an identifier that is not current and passes.
  Why: a supersede never holds a commit on old test comments.
  Costs: stale ties build up and a test can claim a decision that left force.

**Answer:** A

### What does check do with an untested current decision?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, e2533580-1486-4dc9-8697-e6c53605fe72, 41c41584-1ea3-4f15-b7ec-76a1dc99d32f
supersedes: 33a45b9e-8150-488e-be70-7ab2ca9adaa6, 0b962bc8-05a8-48da-9fe8-bf77c7aa6e4a, b871e776-2da5-4f1e-b4c4-fb8229657e25, f2e31cbe-1ba2-4b77-910a-b9664ce0277f
id: 3460d220-e523-448b-aeff-baa8051b7d06
terms: answer=b36113a1-1384-4e4c-a798-8399f4181366, answered=b36113a1-1384-4e4c-a798-8399f4181366, answers=b36113a1-1384-4e4c-a798-8399f4181366, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, current=e2533580-1486-4dc9-8697-e6c53605fe72, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, tier=f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c

Problem: a current decision with no test fails check once its tier is answered, so the last answer of a tier and every test of the tier reach git in one commit, and no answer of a later tier reaches git before the earlier tier is built.
Standard: a report separates a state from a violation; work in progress is counted and shown, and a gate fires only on a rule that is broken.

- A) Check reports it and fails nothing: an untested current decision is a state check prints on every run until a test names the decision.
  Why: an answer reaches git the moment it is given, and the build follows on any tier at any commit.
  Costs: nothing in the tool holds a decision to a test; a decision stays untested for as long as nothing is built on it.
- B) Check fails it: a current decision with no test fails check on every run.
  Why: every commit holds every current decision to a test.
  Costs: an answer cannot reach git before its test, so the answers and the build of a tier land in one commit.
- C) Check fails it once every question of its tier is answered, and reports it as waiting until then.
  Why: the answers of a tier reach git one by one, and a tier is built whole.
  Costs: the last answer of a tier and its build land in one commit, and no answer of a later tier reaches git before the earlier tier is built.

### What does a test carry to name its decision?
depends: 3460d220-e523-448b-aeff-baa8051b7d06=*, 41c41584-1ea3-4f15-b7ec-76a1dc99d32f, e877e4a9-2caf-4fe6-9f82-5476a2abb00b
id: 4319a653-6901-446c-94ab-6420ef472134

### Does check fail a test that names a decision that is not current?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, e2533580-1486-4dc9-8697-e6c53605fe72, 4319a653-6901-446c-94ab-6420ef472134
id: 751ef30c-6a2f-4936-bf9b-b9d487146e53
