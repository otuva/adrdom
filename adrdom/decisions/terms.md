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
