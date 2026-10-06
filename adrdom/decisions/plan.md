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

### What is the shape of a topic file?
depends: 08193e75-dd89-4857-8e9f-b19e1a9e75e6, 277a8338-c57a-43cc-8b28-6b42e9347f6a
unrelated: 94677ed8-eaf6-46e2-bc71-7392fa0805bf, 76bd7ef0-f5be-40d8-b5ba-0bd4bbd047e4, d981f6b0-df0d-428f-8c0e-e220b1651fa1, 292a10b8-c287-4d71-b2bd-840da0a6de61, f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c, aef1b384-2b33-47a2-92a8-ec7f8fb258ef, 46fc51ff-66b9-4841-a953-ae0d4c2fc424, 08a1b488-32fc-4dc7-b4b3-93a6e768725f, 7a1e7b81-0967-4ed2-8a8a-c7153056836f, 75075c78-fe3c-4fd8-8a33-29a5913d9d34, 0a742360-83c2-479d-b858-4bea646c9b36, 6b3d8e59-e145-49e1-860d-a79d753d745c, 178a232e-b51c-4122-a1a9-9e9ed6970748, 5b9170f9-a48e-4ddd-a7dc-73c3be1c2fb8, 23f21671-e34c-44aa-8804-6163d975807f, 8b324fe0-621b-44d9-b7fe-f8e0ee15164e, 3ce82f34-a4fc-4918-bb9b-65a0d81a6853, a82db5b7-969c-44e9-9eec-876ead7099d3, 28b5837d-46ae-459e-ba4c-12265ef07ac1, 05141f49-d9d2-407c-b60c-ebb195a05864
supersedes: f97e62a4-45ab-46c6-b4b1-a674792a4493, b741b573-a2ee-498e-98fd-3a7e7d319470, 055d303b-c286-4f6c-975a-d9d8090936d4, 5892981c-f213-404f-b93f-551e4d081ebf, 923818a7-05c6-4977-bbaf-53ee8f3773ef, 39b76886-98f9-4019-9194-db9ab51aea1e, 32a189a0-86e0-44d1-9392-a50c84570ada, 423065fc-6d9f-40c8-a0bf-8e1779e86b38
id: 9913568f-6b85-4d26-946b-c6e108f9aa51
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, decisions=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, topic=292a10b8-c287-4d71-b2bd-840da0a6de61, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: when the parser skips text it does not recognise, sentences nobody picked sit between decisions in a governed file and read as if they had force.
Standard: structured document formats fix which elements may appear where and reject the rest.

- A) One first-level heading, optional prose, then the questions as third-level headings ending in ?, nothing else.
  Why: every sentence in the file is the topic's prose, the plan, or a question.
  Costs: nothing can be written between questions, not even a note.
- B) Anything: the parser reads what it recognises and skips the rest.
  Why: the agent writes freely.
  Costs: text nobody picked lives between decisions.
- C) The fixed shape, with the prose under the first heading required.
  Why: every topic is explained.
  Costs: writing when there is nothing to say.

**Answer:** A

### Does one question heading stand in two topic files?
depends: 08193e75-dd89-4857-8e9f-b19e1a9e75e6, 277a8338-c57a-43cc-8b28-6b42e9347f6a
supersedes: 76bd7ef0-f5be-40d8-b5ba-0bd4bbd047e4
unrelated: 292a10b8-c287-4d71-b2bd-840da0a6de61, f97e62a4-45ab-46c6-b4b1-a674792a4493, aef1b384-2b33-47a2-92a8-ec7f8fb258ef, 94677ed8-eaf6-46e2-bc71-7392fa0805bf, d981f6b0-df0d-428f-8c0e-e220b1651fa1, b3cb190d-d6f1-4930-bdcd-7827e31ac62a, f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c, 7a1e7b81-0967-4ed2-8a8a-c7153056836f, 055d303b-c286-4f6c-975a-d9d8090936d4, 75075c78-fe3c-4fd8-8a33-29a5913d9d34, f571a454-3e6c-46c1-b754-0822a5d1d667, a82db5b7-969c-44e9-9eec-876ead7099d3, 178a232e-b51c-4122-a1a9-9e9ed6970748, 9913568f-6b85-4d26-946b-c6e108f9aa51, 5b9170f9-a48e-4ddd-a7dc-73c3be1c2fb8, 8b324fe0-621b-44d9-b7fe-f8e0ee15164e, 3ce82f34-a4fc-4918-bb9b-65a0d81a6853, 28b5837d-46ae-459e-ba4c-12265ef07ac1, 05141f49-d9d2-407c-b60c-ebb195a05864
id: 23f21671-e34c-44aa-8804-6163d975807f
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answers=b36113a1-1384-4e4c-a798-8399f4181366, asked=3c8d4c9c-abca-4faa-b7c1-aae39951376c, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, topic=292a10b8-c287-4d71-b2bd-840da0a6de61, topic files=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: the same question asked in two files gets two answers, and an agent cannot tell from the files which one holds.
Standard: ADR collections keep one record per subject.

- A) No.
  Why: one topic per file, and a heading found twice is its exact, cheap sign.
  Costs: the same question worded two ways in two files passes.
- B) Yes.
  Why: nothing compares headings across files.
  Costs: two answers to one question, and the files do not say which holds.

**Answer:** A

### What does check do with a fault on a file awaiting approval?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, 912f4f3b-1213-435a-af0d-7617017fcff4, 34e81f57-9b3c-4dc4-a2ba-ca227523177d
unrelated: 9c3cc17e-33f4-4855-b761-f8f82381cdca, 5fddeeb5-cf54-4550-a701-dd9e4a6a81d6, aa475aa3-05b2-429a-b33a-46b1dcb52049, c10c3a49-e993-4c9f-a46b-4a1ba3646dcf, bf6697e3-8832-4b90-b036-8fecedb55406, 4bb5ef1b-da16-4206-a7d2-c7380c36a4cc, 6ef9eca6-fd8b-4211-aae7-00b37a2a9245, e879fe72-ec07-48cd-9902-2b158aaae726, 751ef30c-6a2f-4936-bf9b-b9d487146e53, 19e1c023-a678-4842-a414-bb0e45c41601, 154e4b9a-c811-4a84-a522-598914eb39f1, c3127af2-4c96-48d5-a164-bac49a9a09ec, a26876ba-5789-4d05-965e-853d00d9edd2, 8b324fe0-621b-44d9-b7fe-f8e0ee15164e, 3ce82f34-a4fc-4918-bb9b-65a0d81a6853, 28b5837d-46ae-459e-ba4c-12265ef07ac1, 5b9170f9-a48e-4ddd-a7dc-73c3be1c2fb8, 178a232e-b51c-4122-a1a9-9e9ed6970748, 9913568f-6b85-4d26-946b-c6e108f9aa51, a82db5b7-969c-44e9-9eec-876ead7099d3, 23f21671-e34c-44aa-8804-6163d975807f
id: 05141f49-d9d2-407c-b60c-ebb195a05864
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approved=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, refuse=0942e445-0163-423e-8ba7-84dd8ac07045

Problem: a draft is in git before the owner reads it; a fault that approve or ask would stop blocks the draft's commit when check fails it, and hides when check is silent.
Standard: a draft gets warnings; the gate fails only what is submitted.

- A) Prints it as a note and fails nothing; the commit of the draft goes through.
  Why: the agent commits a draft at any point, and approve or ask stops the fault before it has force.
  Costs: a broken draft sits in git until approve or ask is run.
- B) Fails it, as on an approved file.
  Why: nothing broken enters git.
  Costs: a draft commits only when every line is right, and the hooks refuse a draft mid-work.

**Answer:** A

### What does approve write for the prose above the first question?
depends: e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, 9913568f-6b85-4d26-946b-c6e108f9aa51=A|C, 61e73795-63de-4669-b09f-48be96919325
unrelated: b741b573-a2ee-498e-98fd-3a7e7d319470, f5eeaacf-1c9f-478b-8a97-2df1aaf39458, fcab4bf9-c02d-4094-b3cb-688c4ec312dc, 277a8338-c57a-43cc-8b28-6b42e9347f6a, f5c242b6-e2f6-45a3-8c61-c743f3ad794f, e4a2ef0c-3354-4c9b-a8d9-9e204e544d85, 7cf81ded-be48-4777-b83e-f8a2998c1ab5, d16be877-9422-4f2e-a6a1-e099be9e3b2b, 46fc51ff-66b9-4841-a953-ae0d4c2fc424, f97e62a4-45ab-46c6-b4b1-a674792a4493, d981f6b0-df0d-428f-8c0e-e220b1651fa1, 94677ed8-eaf6-46e2-bc71-7392fa0805bf, e879fe72-ec07-48cd-9902-2b158aaae726, 7a1e7b81-0967-4ed2-8a8a-c7153056836f, 3ce82f34-a4fc-4918-bb9b-65a0d81a6853, 178a232e-b51c-4122-a1a9-9e9ed6970748, 23f21671-e34c-44aa-8804-6163d975807f, 8b324fe0-621b-44d9-b7fe-f8e0ee15164e, 05141f49-d9d2-407c-b60c-ebb195a05864, a82db5b7-969c-44e9-9eec-876ead7099d3, 28b5837d-46ae-459e-ba4c-12265ef07ac1
id: 5b9170f9-a48e-4ddd-a7dc-73c3be1c2fb8
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approved=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, topic=292a10b8-c287-4d71-b2bd-840da0a6de61, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: the prose at the top of a topic file sets the scope of the whole topic; if nothing covers it an agent can rewrite it at will, and one hash over prose and headings together cannot say which of the two changed.
Standard: a log that hashes named parts separately says what changed; one hash over the whole needs a diff to say the same.

- A) One event of its own, over its own hash, apart from the event for the headings.
  Why: every change to a topic file is approved and named for what it was.
  Costs: a typo in the prose costs an approve.
- B) Nothing of its own: one hash and one event over prose and headings together.
  Why: one event per approve.
  Costs: the ledger cannot say whether the plan or the prose changed.
- C) Nothing: the prose is free text.
  Why: the agent edits it freely.
  Costs: the one place in a topic file where an agent changes governed text at will.

**Answer:** A

### Does ask run on a file whose prose is unapproved?
depends: 3c8d4c9c-abca-4faa-b7c1-aae39951376c, 5b9170f9-a48e-4ddd-a7dc-73c3be1c2fb8=A|B, 7d47885f-3a45-45f9-a092-4314301f0d7b, da59b448-1afb-4469-b40e-69eda8f253c2
unrelated: b741b573-a2ee-498e-98fd-3a7e7d319470, 912f4f3b-1213-435a-af0d-7617017fcff4, d808917b-0717-45ad-ab83-9fa274b9a9d4, aa475aa3-05b2-429a-b33a-46b1dcb52049, 4f04bae4-98e3-48eb-9403-3257268e2d87, 423065fc-6d9f-40c8-a0bf-8e1779e86b38, 9c3cc17e-33f4-4855-b761-f8f82381cdca, 4bb5ef1b-da16-4206-a7d2-c7380c36a4cc, fcab4bf9-c02d-4094-b3cb-688c4ec312dc, 21ae8ae0-6884-4d3c-8464-024a8a5aae2d, 6b3d8e59-e145-49e1-860d-a79d753d745c, 506b3c6d-3d0a-4a02-98de-3b8a573d2fe9, 11be073c-ce91-44d8-833a-ef0b5e0b1672, db58d4b8-9e64-44b0-99a2-acb7f05d5b4e, 644b43ce-be10-4e9a-a7d5-71b27c51e365, 178a232e-b51c-4122-a1a9-9e9ed6970748, 8b324fe0-621b-44d9-b7fe-f8e0ee15164e, 9913568f-6b85-4d26-946b-c6e108f9aa51, 05141f49-d9d2-407c-b60c-ebb195a05864, 23f21671-e34c-44aa-8804-6163d975807f, 28b5837d-46ae-459e-ba4c-12265ef07ac1, a82db5b7-969c-44e9-9eec-876ead7099d3
id: 3ce82f34-a4fc-4918-bb9b-65a0d81a6853
terms: answers=b36113a1-1384-4e4c-a798-8399f4181366, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approved=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, asked=3c8d4c9c-abca-4faa-b7c1-aae39951376c, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a

Problem: a question is asked under prose the owner has not read, and the prose sets the scope the question is read in.
Standard: a change is reviewed whole; a part left unreviewed holds the rest.

- A) No.
  Why: the owner reads a question under prose approved with it.
  Costs: a prose edit holds every question of the file until approve.
- B) Yes.
  Why: a prose edit never holds a question.
  Costs: the owner answers a question under scope text nobody approved.

**Answer:** A

### What stands under a question's heading?
depends: 277a8338-c57a-43cc-8b28-6b42e9347f6a, 43778946-d44c-4acf-a766-0a4915fce199, 9913568f-6b85-4d26-946b-c6e108f9aa51=A|C
unrelated: 055d303b-c286-4f6c-975a-d9d8090936d4, aef1b384-2b33-47a2-92a8-ec7f8fb258ef, 9d634996-475e-4ddc-8113-05c404ad9d5c, f97e62a4-45ab-46c6-b4b1-a674792a4493, ad9761e2-6d64-4b45-b0ba-5c5d8d2b38bf, 946263aa-519a-47aa-b743-5cdcf52f5d35, da59b448-1afb-4469-b40e-69eda8f253c2, 76bd7ef0-f5be-40d8-b5ba-0bd4bbd047e4, 1fb71321-bb84-492f-b9d5-1444c9c444af, b741b573-a2ee-498e-98fd-3a7e7d319470, 38216d39-b064-4927-ab60-951a3c89e388, f5c242b6-e2f6-45a3-8c61-c743f3ad794f, 5b9170f9-a48e-4ddd-a7dc-73c3be1c2fb8, 23f21671-e34c-44aa-8804-6163d975807f, 3ce82f34-a4fc-4918-bb9b-65a0d81a6853, 8b324fe0-621b-44d9-b7fe-f8e0ee15164e, 28b5837d-46ae-459e-ba4c-12265ef07ac1, a82db5b7-969c-44e9-9eec-876ead7099d3, 05141f49-d9d2-407c-b60c-ebb195a05864
id: 178a232e-b51c-4122-a1a9-9e9ed6970748
terms: agents=f90dbd5b-8fbe-4899-8588-f8762795c2e2, options=43778946-d44c-4acf-a766-0a4915fce199, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a

Problem: when a question's body has no fixed parts, agents write long prose that hides the recommendation and invents alternatives, and the owner cannot tell the problem from the options.
Standard: MADR and Nygard templates fix the parts of an architecture record: the context, the options considered, their pros and cons, and the outcome.

- A) A one-line Problem and Standard, then two or three lettered options, each one sentence with Why and Costs lines.
  Why: every question reads the same way, and the options stay short enough to compare.
  Costs: a question that needs more context has nowhere to put it.
- B) The same parts, with free prose allowed between them.
  Why: room for context.
  Costs: prose between the parts is where recommendations and invented alternatives hide.
- C) Only the options; the problem and the standard move to the conversation.
  Why: the shortest question.
  Costs: the record loses why the question exists.

**Answer:** A

### What binds in an answered question?
depends: 43778946-d44c-4acf-a766-0a4915fce199, 41c41584-1ea3-4f15-b7ec-76a1dc99d32f, b36113a1-1384-4e4c-a798-8399f4181366, 178a232e-b51c-4122-a1a9-9e9ed6970748=A|B
unrelated: 9d634996-475e-4ddc-8113-05c404ad9d5c, 277a8338-c57a-43cc-8b28-6b42e9347f6a, 946263aa-519a-47aa-b743-5cdcf52f5d35, 11be073c-ce91-44d8-833a-ef0b5e0b1672, 423065fc-6d9f-40c8-a0bf-8e1779e86b38, 462e22c6-8b23-4061-9a2a-bbf8691fce60, 7a1e7b81-0967-4ed2-8a8a-c7153056836f, f571a454-3e6c-46c1-b754-0822a5d1d667, 7327cd58-ac58-40a4-8ef2-00072fb552b2, 39b76886-98f9-4019-9194-db9ab51aea1e, 32a189a0-86e0-44d1-9392-a50c84570ada, c3127af2-4c96-48d5-a164-bac49a9a09ec, 055d303b-c286-4f6c-975a-d9d8090936d4, b782664f-7465-4e1e-99e0-13f6dd2de774, 0942e445-0163-423e-8ba7-84dd8ac07045, 711a7636-9f4d-402d-ae62-265208adf0ac, f8d92841-aef7-4cd7-bd94-0e07c03e4cb6, 8b324fe0-621b-44d9-b7fe-f8e0ee15164e, 05141f49-d9d2-407c-b60c-ebb195a05864, a82db5b7-969c-44e9-9eec-876ead7099d3, 3ce82f34-a4fc-4918-bb9b-65a0d81a6853, 5b9170f9-a48e-4ddd-a7dc-73c3be1c2fb8, 9913568f-6b85-4d26-946b-c6e108f9aa51, 23f21671-e34c-44aa-8804-6163d975807f
id: 28b5837d-46ae-459e-ba4c-12265ef07ac1
terms: answered=b36113a1-1384-4e4c-a798-8399f4181366, asks=3c8d4c9c-abca-4faa-b7c1-aae39951376c, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, option=43778946-d44c-4acf-a766-0a4915fce199, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a

Problem: a Why or Costs line that states a mechanism reads as a rule, and a builder cannot tell which line the owner's letter made law.
Standard: in a contract the operative clause binds and the recitals explain it; on a conflict the operative clause holds.

- A) The option sentence the owner picked; Why and Costs explain it, and on a clash the option holds.
  Why: one sentence per decision to build from, and a rule has no line left to hide in.
  Costs: a mechanism named only in Costs binds nothing, and the builder derives it or asks.
- B) The option sentence with its Why and Costs lines together.
  Why: nothing the owner read is lost to the builder.
  Costs: three lines of rule per letter, and a clash between them has no tiebreak.
- C) The option sentence and its Costs line; Why explains.
  Why: what the code has to keep is in Costs.
  Costs: two binding lines, and the Costs line turns into a second option.

**Answer:** A

### Does a reference line hold its list on one line?
depends: aef1b384-2b33-47a2-92a8-ec7f8fb258ef, 2bae531b-c4df-48a0-89a0-3d52726741ea, 178a232e-b51c-4122-a1a9-9e9ed6970748=A|B, 7cad9922-9bd1-4c1a-aedc-df1f77c83e1f
unrelated: 5892981c-f213-404f-b93f-551e4d081ebf, 796a2544-43e5-43db-b5d3-8a4e737ec9c6, 32a189a0-86e0-44d1-9392-a50c84570ada, 39b76886-98f9-4019-9194-db9ab51aea1e, f1294391-48b0-4ba3-82b4-c1d6b1c2c884, c6da863d-5085-402f-815a-9bb832b4df7e, 385a57d5-5f76-454a-ae5d-9b0d6f5c1492, 277a8338-c57a-43cc-8b28-6b42e9347f6a, bc4291c8-98f1-499d-8e02-46e371035e70, 055d303b-c286-4f6c-975a-d9d8090936d4, f4f667ad-8714-4e65-897b-c7002e423c28, ad9761e2-6d64-4b45-b0ba-5c5d8d2b38bf, 76bd7ef0-f5be-40d8-b5ba-0bd4bbd047e4, 23f21671-e34c-44aa-8804-6163d975807f, 8b324fe0-621b-44d9-b7fe-f8e0ee15164e, 28b5837d-46ae-459e-ba4c-12265ef07ac1, 5b9170f9-a48e-4ddd-a7dc-73c3be1c2fb8, 9913568f-6b85-4d26-946b-c6e108f9aa51, 3ce82f34-a4fc-4918-bb9b-65a0d81a6853, 05141f49-d9d2-407c-b60c-ebb195a05864
id: a82db5b7-969c-44e9-9eec-876ead7099d3
terms: depends=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, supersedes=2bae531b-c4df-48a0-89a0-3d52726741ea, terms=f590059d-b681-40ec-8a1c-c8916d71f6bb

Problem: a depends, terms or supersedes line spread over several lines, one entry per line, is a second layout for the same job, and each new kind of list invites another.
Standard: one field, one line, entries separated by commas, as in email and HTTP headers.

- A) Yes.
  Why: one rule for every list, and one line per field keeps diffs readable.
  Costs: a long list makes a long line.
- B) No.
  Why: each entry diffs on its own.
  Costs: two layouts to parse, and a list's end is found only by what follows it.

**Answer:** A

### What holds the list of loaded phrases?
depends: 277a8338-c57a-43cc-8b28-6b42e9347f6a, 178a232e-b51c-4122-a1a9-9e9ed6970748
unrelated: a82db5b7-969c-44e9-9eec-876ead7099d3, 08a1b488-32fc-4dc7-b4b3-93a6e768725f, c6da863d-5085-402f-815a-9bb832b4df7e, 94677ed8-eaf6-46e2-bc71-7392fa0805bf, 946263aa-519a-47aa-b743-5cdcf52f5d35, b3cb190d-d6f1-4930-bdcd-7827e31ac62a, aef1b384-2b33-47a2-92a8-ec7f8fb258ef, 528fe6f6-6d77-42d5-9086-017048d1448d, f1294391-48b0-4ba3-82b4-c1d6b1c2c884, 7a1e7b81-0967-4ed2-8a8a-c7153056836f, fcc5b6fe-9072-42ea-a25b-2be72e094bac, 28b5837d-46ae-459e-ba4c-12265ef07ac1, 6108b8b2-911e-443b-abe0-4c6f8b407384, 6b3d8e59-e145-49e1-860d-a79d753d745c, 7d47885f-3a45-45f9-a092-4314301f0d7b, 1b181f10-97f4-4bbe-8939-d5842192126c
id: 149c9a73-16eb-46aa-9806-2950423ee1a2
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answer=b36113a1-1384-4e4c-a798-8399f4181366, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a

Problem: a question that names who spoke, describes the code of the moment, carries a date, opens by assuming an answer, or joins two questions reaches the owner loaded, and the owner is the costliest place to catch it.
Standard: style linters such as Vale run when text is written, with the phrase list kept in the tool.

- A) The tool: the list lives in its code and holds for every repository.
  Why: the same guard runs in every repository and for every agent, before a loaded question reaches the owner.
  Costs: a false hit costs the agent a reword, and changing the list is a code change and a release.
- B) This repository: a file of its own, read by a test over its records.
  Why: nothing is added to the tool.
  Costs: other repositories get no guard, and a loaded question there is caught only when the owner reads it.

**Answer:** A

### What names a topic file?
depends: 08193e75-dd89-4857-8e9f-b19e1a9e75e6, 292a10b8-c287-4d71-b2bd-840da0a6de61, 9913568f-6b85-4d26-946b-c6e108f9aa51
unrelated: 94677ed8-eaf6-46e2-bc71-7392fa0805bf, d981f6b0-df0d-428f-8c0e-e220b1651fa1, 23f21671-e34c-44aa-8804-6163d975807f, aef1b384-2b33-47a2-92a8-ec7f8fb258ef, 7cad9922-9bd1-4c1a-aedc-df1f77c83e1f, 912f4f3b-1213-435a-af0d-7617017fcff4, f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c, 1b181f10-97f4-4bbe-8939-d5842192126c, 796a2544-43e5-43db-b5d3-8a4e737ec9c6, 38216d39-b064-4927-ab60-951a3c89e388
id: 625dea67-13a7-4e8e-913f-e997081a120c
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, topic=292a10b8-c287-4d71-b2bd-840da0a6de61, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6, topics=292a10b8-c287-4d71-b2bd-840da0a6de61

Problem: a file named by a number or a prefix breaks every citation when topics are reordered, and two spellings of one topic make two files.
Standard: ADR collections name each file by a slug of its subject.

- A) Its topic, in lowercase words joined by hyphens, with no number or prefix.
  Why: a name without a number survives reordering, and one spelling per topic.
  Costs: the directory listing shows no order.
- B) A number prefix for order, then its topic.
  Why: the directory listing shows the order topics were opened.
  Costs: renumbering breaks every citation.
- C) Anything the agent picks.
  Why: nothing new in the tool.
  Costs: two spellings of one topic make two files.
