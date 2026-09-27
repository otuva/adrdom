# Terms brief

How the meaning of every term reaches the agent before it writes. The terms file holds identifiers and terms lines the tool needs; the agent needs the names, the aliases and the definitions.

### Does the tool write a terms brief for the agent?
depends: 7d44030a-3589-4f29-aaa6-12fa09908ab8, f90dbd5b-8fbe-4899-8588-f8762795c2e2
id: ec0b9293-916e-4ced-a3d7-3b862ab7b069
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, aliases=1929dc82-4128-4621-af0e-043ac181459f, definition=607cb55f-c0fa-40af-b290-c17f3ba0f812, definitions=607cb55f-c0fa-40af-b290-c17f3ba0f812, identifiers=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, retired=c584a68d-e54a-4ad2-8a6b-9d64ed098ae5, term=f590059d-b681-40ec-8a1c-c8916d71f6bb, terms=f590059d-b681-40ec-8a1c-c8916d71f6bb, terms brief=7d44030a-3589-4f29-aaa6-12fa09908ab8, terms file=528fe6f6-6d77-42d5-9086-017048d1448d

Problem: an agent writes questions, code and chat with term words whose definitions it has never read, because nothing puts them in front of it.
Standard: a shared vocabulary is put in front of every contributor before they write, from one source.

- A) Yes: the tool writes a terms brief from the terms file, a file made for the agent to read.
  Why: the definitions reach the agent as one short file with nothing in it the agent has no use for.
  Costs: one more file, which the tool has to keep equal to the terms file.
- B) No: show prints the name, the aliases and the definition of every term that is not retired.
  Why: no new file, and the brief tells the agent to run show before it builds.
  Costs: the definitions reach the agent only once it runs show, after it has written chat and drafts.
- C) No: the brief tells the agent to read the terms file.
  Why: no new file and no new output.
  Costs: the agent reads identifiers and terms lines it has no use for, and only when it follows the sentence.

**Answer:** A

### What does the terms brief hold?
depends: ec0b9293-916e-4ced-a3d7-3b862ab7b069=A, 7c306235-6243-4d79-b9aa-08ed19662779, 7d44030a-3589-4f29-aaa6-12fa09908ab8
id: f1294391-48b0-4ba3-82b4-c1d6b1c2c884
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, alias=1929dc82-4128-4621-af0e-043ac181459f, aliases=1929dc82-4128-4621-af0e-043ac181459f, definition=607cb55f-c0fa-40af-b290-c17f3ba0f812, identifiers=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, retired=c584a68d-e54a-4ad2-8a6b-9d64ed098ae5, term=f590059d-b681-40ec-8a1c-c8916d71f6bb, terms=f590059d-b681-40ec-8a1c-c8916d71f6bb, terms brief=7d44030a-3589-4f29-aaa6-12fa09908ab8, terms file=528fe6f6-6d77-42d5-9086-017048d1448d

Problem: the terms file carries identifiers and terms lines for the tool, and an agent that loads them at every start pays for text it has no use for.
Standard: a glossary lists each word once, with its other forms and its meaning.

- A) One line per term that is not retired, with its name, its aliases and its definition.
  Why: the agent sees every word bound to a term, and the file has as many lines as there are terms.
  Costs: most aliases are inflections the agent would infer.
- B) One line per term that is not retired, with its name and its definition and no aliases.
  Why: the shortest file.
  Costs: a term whose alias is a different word loses that link.
- C) The entries of the terms file as they stand, identifiers and terms lines included.
  Why: nothing is stripped.
  Costs: the same text is held twice, and the agent loads lines it has no use for.

**Answer:** A

### Does approve write the terms brief?
depends: ec0b9293-916e-4ced-a3d7-3b862ab7b069=A, e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, 7d44030a-3589-4f29-aaa6-12fa09908ab8
id: fcab4bf9-c02d-4094-b3cb-688c4ec312dc
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, approved=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, definitions=607cb55f-c0fa-40af-b290-c17f3ba0f812, install=1b98cdea-dc1b-4789-8f67-68d130d7a49d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, terms brief=7d44030a-3589-4f29-aaa6-12fa09908ab8, terms file=528fe6f6-6d77-42d5-9086-017048d1448d

Problem: a terms brief written at the wrong time shows definitions the owner has not approved, or misses ones the owner has.
Standard: a generated file is written by the step that changes its source.

- A) Yes: approve writes the terms brief every time it runs on the terms file, whether or not the terms file changed.
  Why: the terms brief equals the terms file from the moment of approval, and one run gives a repository its first terms brief.
  Costs: an owner command writes one more file.
- B) No: install writes the terms brief, and the agent runs install after every approval of the terms file.
  Why: the agent writes the file again without the owner.
  Costs: between the approval and the run of install, a new session loads definitions that are out of date.

**Answer:** A

### Does check hold the terms brief to the terms file?
depends: ec0b9293-916e-4ced-a3d7-3b862ab7b069=A, 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, 7d44030a-3589-4f29-aaa6-12fa09908ab8, 528fe6f6-6d77-42d5-9086-017048d1448d
id: ec3f875c-a0f8-4085-8da1-fec9dcb14377
terms: check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, terms brief=7d44030a-3589-4f29-aaa6-12fa09908ab8, terms file=528fe6f6-6d77-42d5-9086-017048d1448d

Problem: a terms brief edited by hand or left behind says something the terms file does not.
Standard: a generated file is verified by generating it again and comparing.

- A) Yes: check builds the terms brief from the terms file again and fails when the file on disk is missing or differs.
  Why: no commit carries a terms brief that differs from the terms file.
  Costs: a commit stops until the terms brief is written again.
- B) Yes, as a note: check reports a terms brief that is missing or differs, and passes.
  Why: no commit stops over a file the tool can write again.
  Costs: a terms brief that is out of date reaches the repository.
- C) No: check never reads the terms brief.
  Why: nothing new in check.
  Costs: a hand edit of the terms brief goes unseen.

**Answer:** A

### Does the brief name the terms brief?
depends: ec0b9293-916e-4ced-a3d7-3b862ab7b069=A, 4f925b13-ad16-48b8-acd1-3e30d6e308b6, 7d44030a-3589-4f29-aaa6-12fa09908ab8
id: adee66f2-c8ff-477f-8ac4-a497f694c912
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, agents=f90dbd5b-8fbe-4899-8588-f8762795c2e2, approval=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, definitions=607cb55f-c0fa-40af-b290-c17f3ba0f812, terms=f590059d-b681-40ec-8a1c-c8916d71f6bb, terms brief=7d44030a-3589-4f29-aaa6-12fa09908ab8, terms file=528fe6f6-6d77-42d5-9086-017048d1448d

Problem: a terms brief that nothing points to is never read.
Standard: an instruction file names the files an agent reads at the start of a session.

- A) Yes: the brief holds a line with @ and the path of the terms brief, which some agents load at the start of a session, and a sentence telling the agent to read that file.
  Why: an agent that follows such a line has the definitions with no step of its own, the sentence reaches the others, and the brief never changes with the terms.
  Costs: on an agent that does not follow the line, the reading rests on the agent following the sentence.
- B) Yes: the brief holds a sentence telling the agent to read the terms brief, with no @ line.
  Why: the same on every agent.
  Costs: no agent has the definitions without a step of its own.
- C) No: the brief section of AGENTS.md holds the text of the terms brief itself.
  Why: every agent that reads AGENTS.md has the definitions.
  Costs: every approval of the terms file changes AGENTS.md.

**Answer:** A
