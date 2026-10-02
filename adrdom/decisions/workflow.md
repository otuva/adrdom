# Workflow

What the brief tells an agent to do between the owner's moves: who drafts a plan and when
building starts.

### Does the agent write the plan?
depends: f90dbd5b-8fbe-4899-8588-f8762795c2e2, 94677ed8-eaf6-46e2-bc71-7392fa0805bf
id: 23d53545-c238-48cc-ae5e-21a2c7aab83e
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approves=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approving=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, topic=292a10b8-c287-4d71-b2bd-840da0a6de61

Problem: someone writes the first list of headings for every topic; if nothing says who, each session settles it alone, and the owner ends up reading sources to write lists.
Standard: spec-driven tools have the agent draft the plan and the human sign it off.

- A) Yes: the agent writes the first draft, and the owner strikes, adds, reorders and approves; the owner may also write a plan and approve it the same way.
  Why: the owner designs by striking and approving, not by reading every source to write the list.
  Costs: the first draft leans toward the questions its source happened to raise.
- B) No: the owner writes the headings, and the agent writes only the bodies under them.
  Why: no agent frames the list at all.
  Costs: the owner does the reading.

**Answer:** A

### Does the agent build on a decision before its tier is fully answered?
depends: f90dbd5b-8fbe-4899-8588-f8762795c2e2, 41c41584-1ea3-4f15-b7ec-76a1dc99d32f, f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c
id: f8d92841-aef7-4cd7-bd94-0e07c03e4cb6
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answer=b36113a1-1384-4e4c-a798-8399f4181366, answered=b36113a1-1384-4e4c-a798-8399f4181366, current=e2533580-1486-4dc9-8697-e6c53605fe72, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, tier=f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c, tiers=f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c, topic=292a10b8-c287-4d71-b2bd-840da0a6de61, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: building on one answer while its siblings in the same tier are open risks code a sibling's answer contradicts; waiting for the whole topic lets the slowest question hold every build.
Standard: staged delivery builds a layer once the layer is specified, not per sentence and not only at the end.

- A) No: a tier is built once every question in it is answered, and nothing in a tier is built while a sibling is open.
  Why: one trigger opens the next tier for questions and the finished tier for code, and siblings often shape the same code.
  Costs: one slow question holds its tier's build as well as the tiers above it.
- B) Yes: a decision is buildable the moment it is answered.
  Why: a current decision holds from the moment it is answered.
  Costs: code is built on part of a tier.
- C) No: building waits until every question in the topic file is answered.
  Why: nothing is built on a topic that may still change.
  Costs: the slowest question holds every build in the topic.

**Answer:** A

### Does the agent build what no current decision covers?
depends: f90dbd5b-8fbe-4899-8588-f8762795c2e2, 41c41584-1ea3-4f15-b7ec-76a1dc99d32f, e2533580-1486-4dc9-8697-e6c53605fe72
id: c3127af2-4c96-48d5-a164-bac49a9a09ec
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answer=b36113a1-1384-4e4c-a798-8399f4181366, answers=b36113a1-1384-4e4c-a798-8399f4181366, asks=3c8d4c9c-abca-4faa-b7c1-aae39951376c, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, current=e2533580-1486-4dc9-8697-e6c53605fe72, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a

Problem: code built before its decision turns the later question into a request to confirm what exists, and the list of what needs deciding passes to the agent.
Standard: spec-driven development builds only what the spec states and sends a gap in the spec back to the spec's owner.

- A) No: the agent writes the question and stops that work until the owner answers, names a current decision that looks wrong and stops, and code built without a decision is caught at review because the tool cannot see it.
  Why: the owner keeps the list of what needs deciding and every answer, and no question is written after the code it asks about.
  Costs: work waits on the owner, and the check against building without a decision is review, not the tool.
- B) Yes: the agent builds, says what it built, and the owner decides it afterwards or has it undone.
  Why: work never waits on an answer.
  Costs: the question comes after the code and asks the owner to confirm what exists.

**Answer:** A

### Does a refusal name the command that clears it?
depends: 0942e445-0163-423e-8ba7-84dd8ac07045, 1d7e59ea-99e3-4d4b-9928-5dabca62723d, f90dbd5b-8fbe-4899-8588-f8762795c2e2
id: b0e865b4-9204-4c55-9f5b-dcb755b3a03b
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, refusal=0942e445-0163-423e-8ba7-84dd8ac07045

Problem: a refusal that names only what failed leaves the agent to guess the next move, and a guess can route around a decision or take a step that belongs to the owner.
Standard: actionable error messages state what failed and what to do next.

- A) Yes: every refusal names what failed, the command that clears it and who runs that command, the agent or the owner.
  Why: the agent's next move is given, and a line ending in the owner's command tells the agent to stop and hand that command over.
  Costs: every new refusal needs a known next move before it can be written.
- B) No: a refusal names what failed, and the agent works out the fix.
  Why: messages stay short and prescribe nothing for cases nobody foresaw.
  Costs: the agent guesses, and nothing marks a step as the owner's.

**Answer:** A

### Does the agent that asks a question build its decision?
depends: f90dbd5b-8fbe-4899-8588-f8762795c2e2, 3c8d4c9c-abca-4faa-b7c1-aae39951376c, 277a8338-c57a-43cc-8b28-6b42e9347f6a, 41c41584-1ea3-4f15-b7ec-76a1dc99d32f, f8d92841-aef7-4cd7-bd94-0e07c03e4cb6
id: 9ba58b6a-bc85-44a5-a720-d8e7a6ff7b31
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, agents=f90dbd5b-8fbe-4899-8588-f8762795c2e2, asks=3c8d4c9c-abca-4faa-b7c1-aae39951376c, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, option=43778946-d44c-4acf-a766-0a4915fce199, question=277a8338-c57a-43cc-8b28-6b42e9347f6a

Problem: the agent that wrote an option builds what it meant, not what the sentence says, and the gap never shows.
Standard: the one who writes a specification is not the one who implements it; the handover tests the text.

- A) No.
  Why: a second agent builds from the record alone, and a sentence that cannot be built from shows itself.
  Costs: two agents per round, and the record must hold everything the builder needs.
- B) Yes.
  Why: one agent, one context, no handover.
  Costs: what the agent meant and never wrote reaches the code unseen.

**Answer:** A

### Does the agent that reviews a build name a decision for each behaviour?
depends: f90dbd5b-8fbe-4899-8588-f8762795c2e2, 41c41584-1ea3-4f15-b7ec-76a1dc99d32f, c3127af2-4c96-48d5-a164-bac49a9a09ec
id: 125ee772-d011-4dab-b53e-97a09ec03198
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, option=43778946-d44c-4acf-a766-0a4915fce199

Problem: code the tool cannot see carries rules no decision gave, and review reads it against the option text only.
Standard: traceability review: every change links to a requirement, and an unlinked change is a finding.

- A) Yes.
  Why: a behaviour with no decision is found at the one place that reads the code.
  Costs: the review is longer, and detail with no decision needs the reviewer to say so.
- B) No.
  Why: review reads for defects only.
  Costs: a rule in the code with no decision passes.

**Answer:** A
