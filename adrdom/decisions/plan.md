# Plan

What a topic file and each question in it hold, what the owner approves, and how long a file
stays open.

### Does the owner approve a plan in one step?
depends: da59b448-1afb-4469-b40e-69eda8f253c2, e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, 94677ed8-eaf6-46e2-bc71-7392fa0805bf
id: f01e20ae-00fc-4ec9-a25b-88a90dffc624
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answers=b36113a1-1384-4e4c-a798-8399f4181366, approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approvals=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approved=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approves=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, asked=3c8d4c9c-abca-4faa-b7c1-aae39951376c, asks=3c8d4c9c-abca-4faa-b7c1-aae39951376c, depends=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, tier=f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c, topic=292a10b8-c287-4d71-b2bd-840da0a6de61

Problem: an agent writes headings, order and bodies in one pass and the owner meets every judgment at once; splitting the plan into approved steps costs the owner a move per step.
Standard: staged review pipelines approve an outline before a draft, and each stage asks the reviewer one question.

- A) Yes: one approval covers the headings and their depends lines; the question bodies are written after it, one tier at a time, with no further approval.
  Why: the owner approves once and then only answers; how many passes the agent takes to reach the plan stays its own work.
  Costs: the owner's glance covers the headings and what they stand on together, with no flat list seen first.
- B) No: three approvals, the headings alone, then the depends lines, then the bodies of each tier.
  Why: each approval is one judgment.
  Costs: at least two extra owner moves per topic before any question is asked.

**Answer:** A

### What does a topic file hold?
depends: 08193e75-dd89-4857-8e9f-b19e1a9e75e6, 277a8338-c57a-43cc-8b28-6b42e9347f6a
id: f97e62a4-45ab-46c6-b4b1-a674792a4493
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, decisions=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, topic=292a10b8-c287-4d71-b2bd-840da0a6de61, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: when the parser skips text it does not recognise, sentences nobody picked sit between decisions in a governed file and read as if they had force.
Standard: structured document formats fix which elements may appear where and reject the rest.

- A) A fixed shape: one first-level heading naming the topic, optional prose under it, then the questions as third-level headings ending in "?"; any other text, such as a note between questions, fails check.
  Why: every sentence in the file is the topic's prose, the plan, or a question.
  Costs: nothing can be written between questions, not even a note.
- B) Anything: the parser reads what it recognises and skips the rest.
  Why: the agent writes freely.
  Costs: text nobody picked lives between decisions.
- C) The fixed shape, with the prose under the first heading required.
  Why: every topic is explained.
  Costs: writing when there is nothing to say.

**Answer:** A

### Does a question appear in two topic files?
depends: 08193e75-dd89-4857-8e9f-b19e1a9e75e6, 277a8338-c57a-43cc-8b28-6b42e9347f6a
id: 76bd7ef0-f5be-40d8-b5ba-0bd4bbd047e4
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answers=b36113a1-1384-4e4c-a798-8399f4181366, asked=3c8d4c9c-abca-4faa-b7c1-aae39951376c, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, topic=292a10b8-c287-4d71-b2bd-840da0a6de61, topic files=08193e75-dd89-4857-8e9f-b19e1a9e75e6, topics=292a10b8-c287-4d71-b2bd-840da0a6de61

Problem: the same question asked in two files gets two answers, and an agent cannot tell from the files which one holds.
Standard: ADR collections keep one record per subject and name each file by a slug of its subject.

- A) No: a question heading that appears in two topic files fails check; each file holds one topic and is named by it in lowercase words joined by hyphens, with no number or prefix.
  Why: the duplicate check is the exact, cheap part of one topic per file, and a name without a number survives reordering.
  Costs: the same question worded two ways in two files passes.
- B) No, and each file name carries a number prefix for order.
  Why: the directory listing shows the order topics were opened.
  Costs: renumbering breaks every citation.
- C) Yes: nothing compares headings across files.
  Why: nothing new in check.
  Costs: two answers to one question go unnoticed.

**Answer:** A

### Does a topic file close?
depends: 08193e75-dd89-4857-8e9f-b19e1a9e75e6, b36113a1-1384-4e4c-a798-8399f4181366
id: d981f6b0-df0d-428f-8c0e-e220b1651fa1
terms: answered=b36113a1-1384-4e4c-a798-8399f4181366, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, topic=292a10b8-c287-4d71-b2bd-840da0a6de61, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: an explicit close needs rules for what is allowed afterwards and for reopening; without one, nothing marks that a topic is finished.
Standard: workflows with a close state need a reopen rule; workflows without one treat done as a property.

- A) No: a topic file is done when every question in its plan is answered, check reports it as done, and a heading added later is a plan change like any other.
  Why: nothing in the tool waits on a close.
  Costs: nothing marks that the owner considered a topic finished.
- B) Yes: an owner command closes it, and adding a heading afterwards needs the owner to reopen it.
  Why: an explicit end.
  Costs: an extra owner move that holds nothing approve does not hold.

**Answer:** A

### Does approve cover the prose above the first question?
depends: f97e62a4-45ab-46c6-b4b1-a674792a4493=A|C, e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd
id: b741b573-a2ee-498e-98fd-3a7e7d319470
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approved=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, topic=292a10b8-c287-4d71-b2bd-840da0a6de61, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: the prose at the top of a topic file sets the scope of the whole topic; if nothing covers it an agent can rewrite it at will, and one hash over prose and headings together cannot say which of the two changed.
Standard: a log that hashes named parts separately says what changed; one hash over the whole needs a diff to say the same.

- A) Yes, as its own part: approve writes one event for the headings and one for the prose, each with its own hash; ask stops a file where either part is unapproved, and check reports it as awaiting approval.
  Why: every change to a topic file is approved and named for what it was.
  Costs: a typo in the prose costs an approve.
- B) Yes, in one hash together with the headings.
  Why: one event per approve.
  Costs: the ledger cannot say whether the plan or the prose changed.
- C) No: the prose is free text.
  Why: the agent edits it freely.
  Costs: the one place in a topic file where an agent changes governed text at will.

**Answer:** A

### Does check fail a plan awaiting approval?
depends: f01e20ae-00fc-4ec9-a25b-88a90dffc624=*, 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de
id: 258107eb-2a90-4ce9-a111-d1bba047d423
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approved=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approves=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a

Problem: a drafted plan has to be committed so the owner reads it in git, and a plan nobody approves must not sit unseen; failing it blocks the draft, and ignoring it hides it.
Standard: linters report drafts and pending items as counts and fail only on what is wrong.

- A) No: check reports a file awaiting approval as a count and passes; a question body under an unapproved heading fails.
  Why: the draft can be committed and read in git, the count keeps a stale plan visible, and bodies still cannot land before approval.
  Costs: the parser tells a heading with a body from one without.
- B) Yes: a plan awaiting approval fails check, so it cannot be committed, and the agent shows it to the owner in conversation.
  Why: nothing unapproved enters git.
  Costs: every draft plan lives outside git until it is approved.
- C) No, and check says nothing about it.
  Why: less output.
  Costs: a plan nobody approves goes unseen.

**Answer:** A

### What does a question hold under its heading?
depends: f97e62a4-45ab-46c6-b4b1-a674792a4493, 277a8338-c57a-43cc-8b28-6b42e9347f6a, 43778946-d44c-4acf-a766-0a4915fce199
id: 055d303b-c286-4f6c-975a-d9d8090936d4
terms: agents=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answer=b36113a1-1384-4e4c-a798-8399f4181366, options=43778946-d44c-4acf-a766-0a4915fce199, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a

Problem: when a question's body has no fixed parts, agents write long prose that hides the recommendation and invents alternatives, and the owner cannot tell the problem from the options.
Standard: MADR and Nygard templates fix the parts of an architecture record: the context, the options considered, their pros and cons, and the outcome.

- A) A one-line Problem, a one-line Standard, then two or three lettered options, each one sentence that responds to the heading, with an indented Why line and an indented Costs line; the answer line comes last, written by the owner's command.
  Why: every question reads the same way, and the options stay short enough to compare.
  Costs: a question that needs more context has nowhere to put it.
- B) The same parts, with free prose allowed between them.
  Why: room for context.
  Costs: prose between the parts is where recommendations and invented alternatives hide.
- C) Only the options; the problem and the standard move to the conversation.
  Why: the shortest question.
  Costs: the record loses why the question exists.

**Answer:** A

### Does a reference line hold its whole list on one line?
depends: 055d303b-c286-4f6c-975a-d9d8090936d4=*, aef1b384-2b33-47a2-92a8-ec7f8fb258ef, 2bae531b-c4df-48a0-89a0-3d52726741ea
id: 5892981c-f213-404f-b93f-551e4d081ebf
terms: answer set=9d634996-475e-4ddc-8113-05c404ad9d5c, depends=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, supersedes=2bae531b-c4df-48a0-89a0-3d52726741ea, terms=f590059d-b681-40ec-8a1c-c8916d71f6bb

Problem: a list spread over several lines, one entry per line, is a second layout for the same job, and each new kind of list invites another.
Standard: one field, one line, entries separated by commas, as in email and HTTP headers.

- A) Yes: every reference list in a question, the depends, terms and supersedes lines alike, is one line of comma-separated entries, and each depends entry keeps its own answer set.
  Why: one rule for every list, and one line per field keeps diffs readable.
  Costs: a long list makes a long line.
- B) No: one entry per line under a field name.
  Why: each entry diffs on its own.
  Costs: two layouts to parse, and a list's end is found only by what follows it.

**Answer:** A

### Does the tool stop loaded phrases in a question?
depends: 055d303b-c286-4f6c-975a-d9d8090936d4=*, e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, 3c8d4c9c-abca-4faa-b7c1-aae39951376c
id: 923818a7-05c6-4977-bbaf-53ee8f3773ef
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answer=b36113a1-1384-4e4c-a798-8399f4181366, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a

Problem: a question that names who spoke, describes the code of the moment, carries a date, opens by assuming an answer, or joins two questions reaches the owner loaded, and the owner is the costliest place to catch it.
Standard: style linters such as Vale run when text is written, with the phrase list kept in the tool.

- A) Yes: a phrase list lives in the tool; approve stops a hit in headings and the prose, ask stops a hit in question bodies, and check reports hits in text that can still change.
  Why: the same guard runs in every repository and for every agent, before a loaded question reaches the owner.
  Costs: a false hit costs the agent a reword, and changing the list is a code change and a release.
- B) No: a test in this repository alone scans its own records.
  Why: nothing is added to the tool.
  Costs: other repositories get no guard, and a loaded question there is caught only when the owner reads it.

**Answer:** A
