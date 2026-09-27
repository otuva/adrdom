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

### What does the terms brief hold?
depends: ec0b9293-916e-4ced-a3d7-3b862ab7b069=A, 7c306235-6243-4d79-b9aa-08ed19662779, 7d44030a-3589-4f29-aaa6-12fa09908ab8
id: f1294391-48b0-4ba3-82b4-c1d6b1c2c884

### Does approve write the terms brief?
depends: ec0b9293-916e-4ced-a3d7-3b862ab7b069=A, e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, 7d44030a-3589-4f29-aaa6-12fa09908ab8
id: fcab4bf9-c02d-4094-b3cb-688c4ec312dc

### Does check hold the terms brief to the terms file?
depends: ec0b9293-916e-4ced-a3d7-3b862ab7b069=A, 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, 7d44030a-3589-4f29-aaa6-12fa09908ab8, 528fe6f6-6d77-42d5-9086-017048d1448d
id: ec3f875c-a0f8-4085-8da1-fec9dcb14377

### Does the brief name the terms brief?
depends: ec0b9293-916e-4ced-a3d7-3b862ab7b069=A, 4f925b13-ad16-48b8-acd1-3e30d6e308b6, 7d44030a-3589-4f29-aaa6-12fa09908ab8
id: adee66f2-c8ff-477f-8ac4-a497f694c912
