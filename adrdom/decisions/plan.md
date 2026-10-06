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
depends: topic file, question
unrelated: plan, Does a question appear in two topic files?, Does a topic file close?, topic, tier, depends, What does the agent do to plan a topic file?, What does the tool keep of the reader's word?, Does a question keep its identifier when its heading is reworded?, Does show print every question?, Does a question carry the agent's lean?, Does a refused question stay in its topic file?, What stands under a question's heading?, What does approve write for the prose above the first question?, Does one question heading stand in two topic files?, What stops a loaded phrase in a question?, Does ask run on a file whose prose is unapproved?, Does a reference line hold its list on one line?, What binds in an answered question?, What does check do with a fault on a file awaiting approval?
supersedes: What does a topic file hold?, Does approve cover the prose above the first question?, What does a question hold under its heading?, Does a reference line hold its whole list on one line?, Does the tool stop loaded phrases in a question?, Does the word limit hold for the Why line of an option?, Does the word limit hold for the Costs line of an option?, Does the word limit read answered text?

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

### Does one question heading stand in two topic files?
depends: topic file, question
supersedes: Does a question appear in two topic files?
unrelated: topic, What does a topic file hold?, depends, plan, Does a topic file close?, Does an option under a heading that opens with Does hold one word?, tier, Does a question keep its identifier when its heading is reworded?, What does a question hold under its heading?, Does show print every question?, Does ask pair the whole question to terms?, Does a reference line hold its list on one line?, What stands under a question's heading?, What is the shape of a topic file?, What does approve write for the prose above the first question?, What stops a loaded phrase in a question?, Does ask run on a file whose prose is unapproved?, What binds in an answered question?, What does check do with a fault on a file awaiting approval?

Problem: the same question asked in two files gets two answers, and an agent cannot tell from the files which one holds.
Standard: ADR collections keep one record per subject.

- A) No.
  Why: one topic per file, and a heading found twice is its exact, cheap sign.
  Costs: the same question worded two ways in two files passes.
- B) Yes.
  Why: nothing compares headings across files.
  Costs: two answers to one question, and the files do not say which holds.

### What does check do with a fault on a file awaiting approval?
depends: check, approve, Does check fail a topic file awaiting approval?, Does check at a commit read the staged files?
unrelated: Does the tool stop while a blame file waits for approval?, Does check hold a blame file to its origin?, Does the tool stop while the terms file waits for approval?, Does check fail a ledger whose rows changed place?, Does check fail a ledger row that was edited?, Does an origin leave the repository while its blame file has an open row?, Does check hold the files install writes?, Does the owner approve a blame file?, Does check fail a test that names a decision that is not current?, Does check look at the ledger before a push?, Does a hook print what check prints on a pass?, Does the agent build what no current decision covers?, Does check in CI compare the ledger with the commit before the push?, What stops a loaded phrase in a question?, Does ask run on a file whose prose is unapproved?, What binds in an answered question?, What does approve write for the prose above the first question?, What stands under a question's heading?, What is the shape of a topic file?, Does a reference line hold its list on one line?, Does one question heading stand in two topic files?

Problem: a draft is in git before the owner reads it; a fault that approve or ask would stop blocks the draft's commit when check fails it, and hides when check is silent.
Standard: a draft gets warnings; the gate fails only what is submitted.

- A) Prints it as a note and fails nothing; the commit of the draft goes through.
  Why: the agent commits a draft at any point, and approve or ask stops the fault before it has force.
  Costs: a broken draft sits in git until approve or ask is run.
- B) Fails it, as on an approved file.
  Why: nothing broken enters git.
  Costs: a draft commits only when every line is right, and the hooks refuse a draft mid-work.

### What does approve write for the prose above the first question?
depends: approve, What is the shape of a topic file?=A|C, Does approve hash each question on its own?
unrelated: Does approve cover the prose above the first question?, Does the agent write the bodies before approve?, Does approve write the terms brief?, question, What does answer write for a removed question?, withdraw, Does approve stop a terms edit that moves a word an answered question pairs?, Does approve accept a terms edit that changes more than one entry?, What does the agent do to plan a topic file?, What does a topic file hold?, Does a topic file close?, plan, Does the owner approve a blame file?, Does a question keep its identifier when its heading is reworded?, Does ask run on a file whose prose is unapproved?, What stands under a question's heading?, Does one question heading stand in two topic files?, What stops a loaded phrase in a question?, What does check do with a fault on a file awaiting approval?, Does a reference line hold its list on one line?, What binds in an answered question?

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

### Does ask run on a file whose prose is unapproved?
depends: ask, What does approve write for the prose above the first question?=A|B, What does ask put to the owner in one run?, Does ask stop a question outside the plan?
unrelated: Does approve cover the prose above the first question?, Does check fail a topic file awaiting approval?, Does ask refuse an option over a word limit?, Does the tool stop while the terms file waits for approval?, Does ban refuse a word that appears in the repository?, Does the word limit read answered text?, Does the tool stop while a blame file waits for approval?, Does an origin leave the repository while its blame file has an open row?, Does approve write the terms brief?, Does approve withdraw an asked question struck from the plan?, Does a refused question stay in its topic file?, Does a refused question hold the tiers above it?, Does an answer carry the version of the question the owner read?, Does the owner retire a term that a question names?, Does the ui perform every owner act?, What stands under a question's heading?, What stops a loaded phrase in a question?, What is the shape of a topic file?, What does check do with a fault on a file awaiting approval?, Does one question heading stand in two topic files?, What binds in an answered question?, Does a reference line hold its list on one line?

Problem: a question is asked under prose the owner has not read, and the prose sets the scope the question is read in.
Standard: a change is reviewed whole; a part left unreviewed holds the rest.

- A) No.
  Why: the owner reads a question under prose approved with it.
  Costs: a prose edit holds every question of the file until approve.
- B) Yes.
  Why: a prose edit never holds a question.
  Costs: the owner answers a question under scope text nobody approved.

### What stands under a question's heading?
depends: question, option, What is the shape of a topic file?=A|C
unrelated: What does a question hold under its heading?, depends, answer set, What does a topic file hold?, What does a depends line name?, Does a depends entry on a question in its file carry an answer set?, Does ask stop a question outside the plan?, Does a question appear in two topic files?, What does the tool derive the tiers from?, Does approve cover the prose above the first question?, What does a blame row name?, What does answer write for a removed question?, What does approve write for the prose above the first question?, Does one question heading stand in two topic files?, Does ask run on a file whose prose is unapproved?, What stops a loaded phrase in a question?, What binds in an answered question?, Does a reference line hold its list on one line?, What does check do with a fault on a file awaiting approval?

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

### What binds in an answered question?
depends: option, decision, answer, What stands under a question's heading?=A|B
unrelated: answer set, question, Does a depends entry on a question in its file carry an answer set?, Does an answer carry the version of the question the owner read?, Does the word limit read answered text?, Does answer remove a question whose answer set lacks the answer?, Does a question keep its identifier when its heading is reworded?, Does ask pair the whole question to terms?, Does answer take a question standing on a question with no decision?, Does the word limit hold for the Why line of an option?, Does the word limit hold for the Costs line of an option?, Does the agent build what no current decision covers?, What does a question hold under its heading?, Does the owner answer a question with a refusal?, refusal, Does ask refuse a question with no term?, Does the agent build on a decision before its tier is fully answered?, What stops a loaded phrase in a question?, What does check do with a fault on a file awaiting approval?, Does a reference line hold its list on one line?, Does ask run on a file whose prose is unapproved?, What does approve write for the prose above the first question?, What is the shape of a topic file?, Does one question heading stand in two topic files?

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

### Does a reference line hold its list on one line?
depends: depends, supersede, What stands under a question's heading?=A|B, Does a reference in a topic file carry the identifier of what it names?
unrelated: Does a reference line hold its whole list on one line?, Does a supersedes line name every current decision standing on its target?, Does the word limit hold for the Costs line of an option?, Does the word limit hold for the Why line of an option?, What does the terms brief hold?, Does a term carry its own banned words?, Does the tool take the order of ledger rows from their time?, question, Does a commit run the reading of the set?, What does a question hold under its heading?, Does one decision supersede several?, What does a depends line name?, Does a question appear in two topic files?, Does one question heading stand in two topic files?, What stops a loaded phrase in a question?, What binds in an answered question?, What does approve write for the prose above the first question?, What is the shape of a topic file?, Does ask run on a file whose prose is unapproved?, What does check do with a fault on a file awaiting approval?

Problem: a depends, terms or supersedes line spread over several lines, one entry per line, is a second layout for the same job, and each new kind of list invites another.
Standard: one field, one line, entries separated by commas, as in email and HTTP headers.

- A) Yes.
  Why: one rule for every list, and one line per field keeps diffs readable.
  Costs: a long list makes a long line.
- B) No.
  Why: each entry diffs on its own.
  Costs: two layouts to parse, and a list's end is found only by what follows it.

### What stops a loaded phrase in a question?
depends: approve, ask, What stands under a question's heading?=*
unrelated: Does the tool stop loaded phrases in a question?, Does the reader's word stop a command?, Does the word limit read answered text?, question, Does a banned word pass when it is only quoted?, Does ask refuse an option over a word limit?, Does approve stop a terms edit that moves a word an answered question pairs?, depends, Does ask stop a question outside the plan?, Does check read the words of an answered question again?, Does the tool stop while the terms file waits for approval?, Does approve withdraw an asked question struck from the plan?, Does the agent build what no current decision covers?, Does a question keep its identifier when its heading is reworded?, Does approve cover the prose above the first question?, Does ask run on a file whose prose is unapproved?, What does approve write for the prose above the first question?, What binds in an answered question?, What is the shape of a topic file?, What does check do with a fault on a file awaiting approval?, Does a reference line hold its list on one line?, Does one question heading stand in two topic files?

Problem: a question that names who spoke, describes the code of the moment, carries a date, opens by assuming an answer, or joins two questions reaches the owner loaded, and the owner is the costliest place to catch it.
Standard: style linters such as Vale run when text is written, with the phrase list kept in the tool.

- A) A phrase list in the tool: approve stops a hit in headings and prose, ask one in a body.
  Why: the same guard runs in every repository and for every agent, before a loaded question reaches the owner.
  Costs: a false hit costs the agent a reword, and changing the list is a code change and a release.
- B) A test in this repository alone, over its own records.
  Why: nothing is added to the tool.
  Costs: other repositories get no guard, and a loaded question there is caught only when the owner reads it.
