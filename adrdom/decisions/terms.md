# Terms

What a term holds, what a question's pairing to terms freezes, and how the terms file changes.

### What does a term hold?
depends: f590059d-b681-40ec-8a1c-c8916d71f6bb
id: 7c306235-6243-4d79-b9aa-08ed19662779
terms: aliases=1929dc82-4128-4621-af0e-043ac181459f, definition=607cb55f-c0fa-40af-b290-c17f3ba0f812, identifier=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, term=f590059d-b681-40ec-8a1c-c8916d71f6bb, topic=292a10b8-c287-4d71-b2bd-840da0a6de61, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: when a term is only a word with a gloss, renaming it breaks every place that used the old word, and nothing ties the meaning to anything that survives the rename.
Standard: ISO 704 and SKOS identify a concept apart from its label, keep former labels attached to it, and have documents reference the identifier.

- A) An identifier, a name, a definition and aliases; the identifier carries the meaning, so the name can change and former names stay as aliases.
  Why: matches the standards field by field, with nothing the tool does not read.
  Costs: nothing in the entry says which topic owns the term or how to test it.
- B) The same, plus a test sentence and an owning topic file for each term.
  Why: the predecessors' glossary showed both mattered to people.
  Costs: two fields the tool never reads, kept by hand.
- C) A name and a definition only; the name is the identity.
  Why: the smallest entry.
  Costs: a rename is a new term, and every pairing to the old name breaks.

**Answer:** A

### What does a definition say?
depends: 607cb55f-c0fa-40af-b290-c17f3ba0f812, f590059d-b681-40ec-8a1c-c8916d71f6bb
id: fcc1c1ec-d8e8-428e-9731-9817cdeb8e76
terms: answered=b36113a1-1384-4e4c-a798-8399f4181366, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, definition=607cb55f-c0fa-40af-b290-c17f3ba0f812, definitions=607cb55f-c0fa-40af-b290-c17f3ba0f812, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, terms=f590059d-b681-40ec-8a1c-c8916d71f6bb, terms file=528fe6f6-6d77-42d5-9086-017048d1448d

Problem: a definition that states behaviour is a decision nobody answered, hidden in the terms file where no question covers it.
Standard: ISO/IEC Directives Part 2 keep requirements out of terminological entries; ISO 704 defines by a broader concept plus distinguishing characteristics.

- A) What the thing is, never what it does, in one or two sentences; its words are paired to terms like a question's, and a definition that leads back to itself through other definitions is stopped.
  Why: behaviour lives only in answered questions, and a circle means an entry is not defined plainly enough.
  Costs: some entries need plainer words to break a circle.
- B) Anything that explains the word, behaviour included.
  Why: one place to read what a word means and does.
  Costs: behaviour enters the repository without a question.

**Answer:** A

### Does check read the words of an answered question again?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, b36113a1-1384-4e4c-a798-8399f4181366, f590059d-b681-40ec-8a1c-c8916d71f6bb
id: 6108b8b2-911e-443b-abe0-4c6f8b407384
terms: answered=b36113a1-1384-4e4c-a798-8399f4181366, asked=3c8d4c9c-abca-4faa-b7c1-aae39951376c, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, identifier=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, identifiers=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, retired=c584a68d-e54a-4ad2-8a6b-9d64ed098ae5, stands on=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, terms=f590059d-b681-40ec-8a1c-c8916d71f6bb, terms file=528fe6f6-6d77-42d5-9086-017048d1448d

Problem: if the words of an answered question are matched against the terms file as it stands on every run, a later edit to the terms file changes what an old decision means without any byte of it changing.
Standard: frozen references; a record stores the identifiers it resolved when written, and later reads use those identifiers, not a fresh lookup.

- A) No: check reads only the question's terms line; every identifier on it must exist and not be retired, and the words off the line are never looked at again.
  Why: an answered question means what it meant when asked, whatever the terms file becomes.
  Costs: a word that should have paired and did not stays unpaired for good.
- B) Yes: the words are paired again against the terms file on every run, and any difference fails.
  Why: the records always agree with the terms file.
  Costs: every edit to the terms file can fail answered questions nobody may edit.

**Answer:** A

### Does a term word keep its plain sense in the records?
depends: f590059d-b681-40ec-8a1c-c8916d71f6bb, 1929dc82-4128-4621-af0e-043ac181459f
id: 521e6211-344a-4b56-a1aa-07be956b3b29
terms: aliases=1929dc82-4128-4621-af0e-043ac181459f, answered=b36113a1-1384-4e4c-a798-8399f4181366, answers=b36113a1-1384-4e4c-a798-8399f4181366, approved=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, term=f590059d-b681-40ec-8a1c-c8916d71f6bb

Problem: a word that is both a term and plain English lets a sentence slide between the two meanings, and a reader cannot tell which one the owner answered.
Standard: controlled languages such as ASD-STE100 give each approved word one meaning and list the senses it may not carry.

- A) No: a term word always means the term in the records; its plain sense leaves them, forms that mean the term are aliases, and plain use of the same form is caught at review.
  Why: one word, one meaning, in every sentence the owner answers.
  Costs: some plain words are lost to the records and need a synonym.
- B) Yes: context decides which sense a word carries.
  Why: sentences read naturally.
  Costs: the pairing says the term while the sentence may mean the plain word.

**Answer:** A

### Does ask pair the whole question to terms?
depends: 7c306235-6243-4d79-b9aa-08ed19662779=*, 3c8d4c9c-abca-4faa-b7c1-aae39951376c, 277a8338-c57a-43cc-8b28-6b42e9347f6a
id: f571a454-3e6c-46c1-b754-0822a5d1d667
terms: ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, option=43778946-d44c-4acf-a766-0a4915fce199, options=43778946-d44c-4acf-a766-0a4915fce199, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, terms=f590059d-b681-40ec-8a1c-c8916d71f6bb

Problem: pairing only the chosen sentence leaves the heading, the problem line and the other options free to use a word in another sense, and a question read as a whole can mean something its pairing does not show.
Standard: terminology tools scan the whole document, not one sentence of it.

- A) Yes: ask pairs every word of the question, the heading, the body and every option, to terms, and writes one terms line under it.
  Why: the owner reads the whole question, so the whole question is held to one vocabulary.
  Costs: a slip in a line the choice does not rest on is frozen with the rest.
- B) No: only the heading and the options are paired.
  Why: the lines that state the fork carry the meaning.
  Costs: the problem and standard lines can drift from the terms.

**Answer:** A

### Does the owner approve the terms file as a whole?
depends: 7c306235-6243-4d79-b9aa-08ed19662779=*, e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, 528fe6f6-6d77-42d5-9086-017048d1448d
id: 767a1402-b079-4565-bf21-467236732478
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approves=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, current=e2533580-1486-4dc9-8697-e6c53605fe72, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, decisions=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, definition=607cb55f-c0fa-40af-b290-c17f3ba0f812, options=43778946-d44c-4acf-a766-0a4915fce199, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, retire=c584a68d-e54a-4ad2-8a6b-9d64ed098ae5, supersede=2bae531b-c4df-48a0-89a0-3d52726741ea, term=f590059d-b681-40ec-8a1c-c8916d71f6bb, terms file=528fe6f6-6d77-42d5-9086-017048d1448d

Problem: vocabulary that moves without the owner is the same failure as a decision that moves, but a term has no options, so a question per term turns every rename into a supersede.
Standard: governed glossaries take changes as diffs signed off by a named approver, with tests on shape, uniqueness and references.

- A) Yes: the agent edits the terms file and the owner approves the whole file with approve, which writes one event over its hash; check fails a terms file that differs from its latest approval, ask stops while it does, and on a definition change approve prints the current decisions paired to that term.
  Why: one command the owner has, one event, and one glance at a diff where a new noun stands out.
  Costs: one approval can cover many term edits at once.
- B) No: separate owner commands add, rename and retire a term, each writing its own event.
  Why: each change is its own act.
  Costs: three commands for what a diff shows.
- C) No: the agent edits freely and check holds only shape and uniqueness.
  Why: nothing for the owner to do.
  Costs: vocabulary moves under the owner.

**Answer:** A

### Does approve stop a terms edit that moves a word an answered question pairs?
depends: 6108b8b2-911e-443b-abe0-4c6f8b407384=A, e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd
id: 7cf81ded-be48-4777-b83e-f8a2998c1ab5
terms: answered=b36113a1-1384-4e4c-a798-8399f4181366, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, current=e2533580-1486-4dc9-8697-e6c53605fe72, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, decisions=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, identifiers=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, supersedes=2bae531b-c4df-48a0-89a0-3d52726741ea, term=f590059d-b681-40ec-8a1c-c8916d71f6bb, terms=f590059d-b681-40ec-8a1c-c8916d71f6bb, terms file=528fe6f6-6d77-42d5-9086-017048d1448d

Problem: check reads only frozen identifiers, so an edit that moves a word from one term to another passes check while every decision using that word now reads differently to anyone who looks the word up.
Standard: referential integrity; a change that would leave a stored reference meaning something else is rejected at the point of change.

- A) Yes: approve stops a terms edit that takes a word off an entry while a current decision pairs the word to it, and names those decisions; the owner supersedes them first or leaves the word where it is.
  Why: the terms file cannot quietly change what a current decision reads as.
  Costs: some renames wait for supersedes.
- B) No: approve accepts the move, and the frozen identifiers keep the old meaning.
  Why: vocabulary moves freely.
  Costs: a decision's word and its paired meaning part ways, and only the terms line shows it.

**Answer:** A

### Does a retired term keep its words?
depends: 7c306235-6243-4d79-b9aa-08ed19662779=*, c584a68d-e54a-4ad2-8a6b-9d64ed098ae5
id: 05af74a6-36be-4923-b66f-1467b323816a
terms: answered=b36113a1-1384-4e4c-a798-8399f4181366, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, identifier=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, retired=c584a68d-e54a-4ad2-8a6b-9d64ed098ae5, retiring=c584a68d-e54a-4ad2-8a6b-9d64ed098ae5, term=f590059d-b681-40ec-8a1c-c8916d71f6bb, terms=f590059d-b681-40ec-8a1c-c8916d71f6bb

Problem: splitting a term or merging two needs the old one retired; if it keeps its words, the new terms cannot take them and new questions cannot use them.
Standard: terminology systems let a deprecated term's label pass to a new concept while old records still point at the old identifier.

- A) No: retiring frees its words and a new term may take them; answered questions keep the retired term's identifier on their terms line, so their meaning does not move.
  Why: the right word passes to the new term on a split or merge.
  Costs: one word can mean one thing in an old decision and another in a new one, and the terms line says which.
- B) Yes: the word stays the retired term's for good; no new question may use it and no new term may take it.
  Why: a word never carries two meanings anywhere in the records.
  Costs: a split or merge loses the word.

**Answer:** A

### Does ask refuse a question with no term?
depends: f571a454-3e6c-46c1-b754-0822a5d1d667, 3c8d4c9c-abca-4faa-b7c1-aae39951376c, 277a8338-c57a-43cc-8b28-6b42e9347f6a, f590059d-b681-40ec-8a1c-c8916d71f6bb
id: 711a7636-9f4d-402d-ae62-265208adf0ac
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answer=b36113a1-1384-4e4c-a798-8399f4181366, answered=b36113a1-1384-4e4c-a798-8399f4181366, approves=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, asked=3c8d4c9c-abca-4faa-b7c1-aae39951376c, asks=3c8d4c9c-abca-4faa-b7c1-aae39951376c, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, refuse=0942e445-0163-423e-8ba7-84dd8ac07045, refuses=0942e445-0163-423e-8ba7-84dd8ac07045, stands on=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, term=f590059d-b681-40ec-8a1c-c8916d71f6bb, terms=f590059d-b681-40ec-8a1c-c8916d71f6bb

Problem: a question whose words pair to no term gets an empty terms line, so nothing fixes what its words mean and check has nothing to hold after the answer.
Standard: controlled vocabulary indexing gives every indexed record at least one term from the vocabulary.

- A) Yes: ask refuses a question none of whose words pair to a term, and the agent rewords it or adds a term the owner approves.
  Why: every answered question carries at least one term, so its meaning is fixed and check can hold it.
  Costs: a question on a new subject waits for its term.
- B) No: ask asks the question with an empty terms line.
  Why: a new subject is asked without waiting for a term.
  Costs: the answer stands on words whose meaning nothing fixes.

**Answer:** A

### Does check fail a terms file that differs from its latest approval?
depends: 7cf81ded-be48-4777-b83e-f8a2998c1ab5, 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, 528fe6f6-6d77-42d5-9086-017048d1448d, 607cb55f-c0fa-40af-b290-c17f3ba0f812
id: 03b2853b-4379-4f4b-9cbf-ec6e06aad7fd
terms: agents=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answered=b36113a1-1384-4e4c-a798-8399f4181366, approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approved=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approves=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, current=e2533580-1486-4dc9-8697-e6c53605fe72, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, decisions=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, definition=607cb55f-c0fa-40af-b290-c17f3ba0f812, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, term=f590059d-b681-40ec-8a1c-c8916d71f6bb, terms=f590059d-b681-40ec-8a1c-c8916d71f6bb, terms file=528fe6f6-6d77-42d5-9086-017048d1448d, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6
supersedes: 767a1402-b079-4565-bf21-467236732478

Problem: failing every commit while the terms file waits for approval holds work that has nothing to do with terms, yet a changed definition under answered questions moves what those decisions mean.
Standard: change control gates the step that uses a change, such as merge, submit or apply, not the recording of a draft.

- A) Only when the definition of a term a current decision pairs, directly or through another term's definition, differs from its approved text; otherwise check reports the file as waiting, ask stops until the owner approves the whole file with approve, and approve writes one event over its hash and prints the current decisions paired to a changed term.
  Why: a draft term is committed like a draft plan, and the one edit that moves an answered decision's meaning still stops.
  Costs: check needs each term's approved text and not only the file hash, and a typo in a paired definition still waits for the owner.
- B) Yes: check fails every terms file that differs from its latest approval, whether or not a topic file exists, and approve and ask work as in A.
  Why: nothing in the terms file is committed without the owner.
  Costs: every commit waits while a terms edit waits for approval, including work that has nothing to do with terms.
- C) No: check only reports a terms file that differs from its latest approval, and approve and ask work as in A.
  Why: a terms draft never holds a commit.
  Costs: a changed definition under answered questions lands unapproved, and agents build on it.

**Answer:** A

### Does answer stop while the terms file waits for approval?
depends: b36113a1-1384-4e4c-a798-8399f4181366, 528fe6f6-6d77-42d5-9086-017048d1448d, 03b2853b-4379-4f4b-9cbf-ec6e06aad7fd
id: cd450764-bacd-4378-82ba-19edfc5a9edb
terms: answer=b36113a1-1384-4e4c-a798-8399f4181366, answers=b36113a1-1384-4e4c-a798-8399f4181366, approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approved=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approves=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, checks=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, current=e2533580-1486-4dc9-8697-e6c53605fe72, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, terms=f590059d-b681-40ec-8a1c-c8916d71f6bb, terms file=528fe6f6-6d77-42d5-9086-017048d1448d, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: a terms edit waiting for approval blocks answers to questions whose terms it does not touch.
Standard: change control gates the step that uses a change, not steps that never read it.

- A) Yes: answer stops until the owner approves the terms file.
  Why: every answer is written while the terms file matches its approval.
  Costs: a terms edit for one topic file holds every open answer in the others.
- B) No: answer checks the question's terms line against the approved terms and proceeds.
  Why: an unrelated terms edit never holds an answer, and approve still stops an edit that moves a word a current decision pairs.
  Costs: the owner may answer while a terms edit waits unread.

**Answer:** B

### Does the tool stop while the terms file waits for approval?
depends: 7cf81ded-be48-4777-b83e-f8a2998c1ab5, 528fe6f6-6d77-42d5-9086-017048d1448d, 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, b36113a1-1384-4e4c-a798-8399f4181366
id: aa475aa3-05b2-429a-b33a-46b1dcb52049
terms: answer=b36113a1-1384-4e4c-a798-8399f4181366, approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approved=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approves=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, depends=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, terms=f590059d-b681-40ec-8a1c-c8916d71f6bb, terms file=528fe6f6-6d77-42d5-9086-017048d1448d
supersedes: 03b2853b-4379-4f4b-9cbf-ec6e06aad7fd, cd450764-bacd-4378-82ba-19edfc5a9edb

Problem: while a terms edit waits for approval, any command that reads terms needs a second copy of the approved text to work from.
Standard: a single source of truth; a pending change is either approved or blocks what depends on it.

- A) Yes: ask, answer and check stop while the terms file differs from its latest approval, and the owner approves it first.
  Why: the terms file is the one source, and nothing keeps or looks up an older copy of it.
  Costs: an unrelated terms edit holds every answer and every commit until the owner approves it.
- B) No: the tool keeps the approved text beside the ledger and works from it while the edit waits.
  Why: an unrelated terms edit never holds an answer or a commit.
  Costs: a second copy of the terms file, and the code that keeps it in step with the ledger.

**Answer:** A

### Does the owner retire a term that a question names?
depends: 05af74a6-36be-4923-b66f-1467b323816a=A, c584a68d-e54a-4ad2-8a6b-9d64ed098ae5, f590059d-b681-40ec-8a1c-c8916d71f6bb, 277a8338-c57a-43cc-8b28-6b42e9347f6a
id: db58d4b8-9e64-44b0-99a2-acb7f05d5b4e
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answer=b36113a1-1384-4e4c-a798-8399f4181366, answered=b36113a1-1384-4e4c-a798-8399f4181366, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approved=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, asked=3c8d4c9c-abca-4faa-b7c1-aae39951376c, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, current=e2533580-1486-4dc9-8697-e6c53605fe72, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, decisions=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, refuses=0942e445-0163-423e-8ba7-84dd8ac07045, retire=c584a68d-e54a-4ad2-8a6b-9d64ed098ae5, retired=c584a68d-e54a-4ad2-8a6b-9d64ed098ae5, supersede=2bae531b-c4df-48a0-89a0-3d52726741ea, superseded=2bae531b-c4df-48a0-89a0-3d52726741ea, term=f590059d-b681-40ec-8a1c-c8916d71f6bb

Problem: a term is retired while questions name it; check then fails them, ask has nothing to ask, answer refuses, and no command stops the retire itself.
Standard: a record that others reference is not removed while the references stand.

- A) No: approve refuses the retire while any question that is answered, asked, or a heading of an approved plan names the term, and names those questions.
  Why: no question loses a word under it; the move is a supersede or a plan change first.
  Costs: approve pairs the words of headings whose body is not asked; a term in wide use is retired only once every question naming it is superseded or struck.
- B) No while a current decision pairs it; an asked question or a plan heading that names it is reworded by the agent and asked again.
  Why: the guard covers current decisions, and an open question is the agent's to reword.
  Costs: the owner's retire lands the agent with rework: an asked question's version moves and it is asked again, a plan heading changes and the plan is approved again.
- C) Yes: retire is free, and check reports every question that names a retired term.
  Why: the owner's word on a term is not held by questions.
  Costs: the records fail after an owner command, and the move to clear them is a supersede of each named decision.

**Answer:** A

### Does approve accept a terms edit that changes more than one entry?
depends: e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, 528fe6f6-6d77-42d5-9086-017048d1448d, f590059d-b681-40ec-8a1c-c8916d71f6bb, aa475aa3-05b2-429a-b33a-46b1dcb52049
id: d16be877-9422-4f2e-a6a1-e099be9e3b2b
terms: approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, definition=607cb55f-c0fa-40af-b290-c17f3ba0f812, definitions=607cb55f-c0fa-40af-b290-c17f3ba0f812, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, refused=0942e445-0163-423e-8ba7-84dd8ac07045, terms=f590059d-b681-40ec-8a1c-c8916d71f6bb, terms file=528fe6f6-6d77-42d5-9086-017048d1448d

Problem: ten definitions in one approve are not read one by one, and a definition that states a rule passes.
Standard: a change is reviewed as one unit, and a batch hides the item that should have been refused.

- A) No.
  Why: the owner reads one definition at each approve, and a definition that states a rule stands alone.
  Costs: a repository's first terms file needs one approve per entry.
- B) Yes.
  Why: one approve covers a whole edit, as it does for a plan.
  Costs: the entry that should have been refused passes in the batch.

**Answer:** A

### Does a retired term hold its words?
depends: c584a68d-e54a-4ad2-8a6b-9d64ed098ae5, f590059d-b681-40ec-8a1c-c8916d71f6bb, 7c306235-6243-4d79-b9aa-08ed19662779
supersedes: 05af74a6-36be-4923-b66f-1467b323816a, db58d4b8-9e64-44b0-99a2-acb7f05d5b4e
unrelated: f1294391-48b0-4ba3-82b4-c1d6b1c2c884, 521e6211-344a-4b56-a1aa-07be956b3b29, c96518bf-470d-45c0-8b86-eaf8f54cb490, b3cb190d-d6f1-4930-bdcd-7827e31ac62a, 0487a1cf-eb61-4bf8-bd16-eed899d22d2c, 7d44030a-3589-4f29-aaa6-12fa09908ab8, 735a32bd-db0c-428e-b65e-2acc750671b0, 1b181f10-97f4-4bbe-8939-d5842192126c, 6108b8b2-911e-443b-abe0-4c6f8b407384, 7cf81ded-be48-4777-b83e-f8a2998c1ab5, ec3f875c-a0f8-4085-8da1-fec9dcb14377, fcf9d2c6-009e-4d6a-a3e4-14274bd1ee94
id: 5577f646-a9d4-4785-9193-2317b9aa502d
terms: identifier=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, retired=c584a68d-e54a-4ad2-8a6b-9d64ed098ae5, term=f590059d-b681-40ec-8a1c-c8916d71f6bb, terms=f590059d-b681-40ec-8a1c-c8916d71f6bb

Problem: splitting a term or merging two needs the old one retired; if it holds its words, the new terms cannot take them and new questions cannot use them.
Standard: terminology systems let a deprecated term's label pass to a new concept while old records still point at the old identifier.

- A) No.
  Why: the right word passes to the new term on a split or merge.
  Costs: one word may mean one thing in git history and another in the records.
- B) Yes.
  Why: a word never carries two meanings anywhere in the records.
  Costs: a split or merge loses the word.

**Answer:** A

### What holds a retire of a term?
depends: c584a68d-e54a-4ad2-8a6b-9d64ed098ae5, f590059d-b681-40ec-8a1c-c8916d71f6bb, 277a8338-c57a-43cc-8b28-6b42e9347f6a, e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, 5577f646-a9d4-4785-9193-2317b9aa502d=A
unrelated: 05af74a6-36be-4923-b66f-1467b323816a, f1294391-48b0-4ba3-82b4-c1d6b1c2c884, db58d4b8-9e64-44b0-99a2-acb7f05d5b4e, 7d44030a-3589-4f29-aaa6-12fa09908ab8, 7c306235-6243-4d79-b9aa-08ed19662779, 521e6211-344a-4b56-a1aa-07be956b3b29, 0487a1cf-eb61-4bf8-bd16-eed899d22d2c, ec3f875c-a0f8-4085-8da1-fec9dcb14377, c96518bf-470d-45c0-8b86-eaf8f54cb490, 711a7636-9f4d-402d-ae62-265208adf0ac, b0e865b4-9204-4c55-9f5b-dcb755b3a03b, 6b3d8e59-e145-49e1-860d-a79d753d745c, 1b181f10-97f4-4bbe-8939-d5842192126c, 6108b8b2-911e-443b-abe0-4c6f8b407384, aef1b384-2b33-47a2-92a8-ec7f8fb258ef
id: fcf9d2c6-009e-4d6a-a3e4-14274bd1ee94
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, answer=b36113a1-1384-4e4c-a798-8399f4181366, answered=b36113a1-1384-4e4c-a798-8399f4181366, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approved=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, asked=3c8d4c9c-abca-4faa-b7c1-aae39951376c, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, current=e2533580-1486-4dc9-8697-e6c53605fe72, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, decisions=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, refuses=0942e445-0163-423e-8ba7-84dd8ac07045, retire=c584a68d-e54a-4ad2-8a6b-9d64ed098ae5, retired=c584a68d-e54a-4ad2-8a6b-9d64ed098ae5, supersede=2bae531b-c4df-48a0-89a0-3d52726741ea, superseded=2bae531b-c4df-48a0-89a0-3d52726741ea, term=f590059d-b681-40ec-8a1c-c8916d71f6bb

Problem: a term is retired while questions name it; check then fails them, ask has nothing to ask, answer refuses, and no command stops the retire itself.
Standard: a record that others reference is not removed while the references stand.

- A) Every question that names it: answered, asked, or a heading of an approved plan; approve refuses and names them.
  Why: no question loses a word under it; the move is a supersede or a plan change first.
  Costs: a term in wide use is retired only once every question naming it is superseded or struck.
- B) A current decision that names it alone; an asked question or plan heading is reworded and asked again.
  Why: the guard covers current decisions, and an open question is the agent's to reword.
  Costs: the owner's retire lands the agent with rework, a version moves, a plan is approved again.
- C) Nothing: retire is free, and check reports every question that names a retired term.
  Why: the owner's word on a term is not held by questions.
  Costs: the records fail after an owner command, and the move to clear them is a supersede of each named decision.

**Answer:** A
