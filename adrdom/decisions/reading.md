# Reading

The current decisions read against each other: the same reader and the same neighbours as planning, run over the set that exists instead of over a new question.

### Does the tool read every current decision against its neighbours?
depends: 41c41584-1ea3-4f15-b7ec-76a1dc99d32f, daec9ffd-676f-4111-a359-2827bfd4527c, be0d6f5c-ae1b-4753-8ec7-0f4b03d84641, de8ef568-3a8d-42ba-bef7-d7fd467b27be, 08a1b488-32fc-4dc7-b4b3-93a6e768725f, dad5273c-afe2-4ab1-a451-e2a5d4d9a26d
unrelated: 4838baa4-a5df-49e7-929c-ed63b5adaf4e, 97c679f6-e8e6-4970-b547-940477753d7d, 385a57d5-5f76-454a-ae5d-9b0d6f5c1492, c3127af2-4c96-48d5-a164-bac49a9a09ec, 796a2544-43e5-43db-b5d3-8a4e737ec9c6, c6451ecc-17a9-447e-a9c9-fe9b35ec789c, 644b43ce-be10-4e9a-a7d5-71b27c51e365, 48f4647d-8c81-4e8a-a936-6b32f5fb0699, 125ee772-d011-4dab-b53e-97a09ec03198, 6108b8b2-911e-443b-abe0-4c6f8b407384, f4f667ad-8714-4e65-897b-c7002e423c28, bf6697e3-8832-4b90-b036-8fecedb55406, 751ef30c-6a2f-4936-bf9b-b9d487146e53, ce19c883-c61d-477b-a6d4-fee5134c6e9a, cd83e277-ebf0-4458-a77a-c526ab4cfeca, 908bddc6-5d30-4bcc-be8d-4c470c6bff11, 4b407ee1-1e64-4ca1-842b-69773fc0e48a, bc4291c8-98f1-499d-8e02-46e371035e70, 2db33118-82b4-4edf-a9cb-e92ac62738b9
id: 1ab6d6c3-7e44-42de-bc2f-2e5849114aef
terms: current=e2533580-1486-4dc9-8697-e6c53605fe72, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, decisions=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, identifier=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, neighbour=daec9ffd-676f-4111-a359-2827bfd4527c, neighbours=daec9ffd-676f-4111-a359-2827bfd4527c, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, reader=be0d6f5c-ae1b-4753-8ec7-0f4b03d84641

Problem: the set holds decisions that repeat or contradict each other and nothing reads them together; a new question meets them, two old ones never meet.
Standard: periodic deduplication of a store with the same comparator used at insert.

- A) Yes.
  Why: the pairs planning never sees, old against old, get the same word and reason as a new question's neighbours.
  Costs: ten reader calls per current decision; rows in the reader's file keyed by text hash and neighbour identifier.
- B) No.
  Why: the set is read only where a new question touches it; no cost beyond planning.
  Costs: known duplicates and contradictions in the set stay until a new question lands beside them.

### What does the reader say about two current decisions?
depends: be0d6f5c-ae1b-4753-8ec7-0f4b03d84641, e2533580-1486-4dc9-8697-e6c53605fe72, 41c41584-1ea3-4f15-b7ec-76a1dc99d32f, aef1b384-2b33-47a2-92a8-ec7f8fb258ef, 1ab6d6c3-7e44-42de-bc2f-2e5849114aef=A, c6e0b7d6-cdec-46da-b2bc-cda218673868
unrelated: 277a8338-c57a-43cc-8b28-6b42e9347f6a, 2bae531b-c4df-48a0-89a0-3d52726741ea, de5ed5b1-3a83-4403-8953-941cb7e94390, f4f667ad-8714-4e65-897b-c7002e423c28, 62e50382-db1b-4649-9f26-7f44c379d6fe, dad5273c-afe2-4ab1-a451-e2a5d4d9a26d, 39b76886-98f9-4019-9194-db9ab51aea1e, 9d634996-475e-4ddc-8113-05c404ad9d5c, e3200081-9995-4bbe-983f-2f430dc341e7, ad9761e2-6d64-4b45-b0ba-5c5d8d2b38bf, 055d303b-c286-4f6c-975a-d9d8090936d4, 4b407ee1-1e64-4ca1-842b-69773fc0e48a, bc4291c8-98f1-499d-8e02-46e371035e70, 2db33118-82b4-4edf-a9cb-e92ac62738b9
id: 908bddc6-5d30-4bcc-be8d-4c470c6bff11

Problem: both sides are answered, so the whole relation can be read; the words must be ones the agent can act on, and the pair has no new side.
Standard: the same closed set as for a question with its body, applied to two settled texts.

- A) One of same, consistent, inconsistent, depends, unrelated, after one sentence of reason.
  Why: one set for the reader, one file of rows, the same moves: same and inconsistent become a supersede question.
  Costs: depends between two decisions is a finding with no line to write; it is printed and left.
- B) One of same, inconsistent, unrelated, after one sentence of reason.
  Why: only the words that lead to a move; fewer wrong words to read.
  Costs: a missing tie between two old decisions is never named; the set keeps its untied pairs.

### Does the tool read a pair of decisions in both directions?
depends: be0d6f5c-ae1b-4753-8ec7-0f4b03d84641, 41c41584-1ea3-4f15-b7ec-76a1dc99d32f, 1ab6d6c3-7e44-42de-bc2f-2e5849114aef=A, 908bddc6-5d30-4bcc-be8d-4c470c6bff11=A
unrelated: f4f667ad-8714-4e65-897b-c7002e423c28, 4838baa4-a5df-49e7-929c-ed63b5adaf4e, e3200081-9995-4bbe-983f-2f430dc341e7, dad5273c-afe2-4ab1-a451-e2a5d4d9a26d, 39b76886-98f9-4019-9194-db9ab51aea1e, 277a8338-c57a-43cc-8b28-6b42e9347f6a, 385a57d5-5f76-454a-ae5d-9b0d6f5c1492, c3127af2-4c96-48d5-a164-bac49a9a09ec, 32a189a0-86e0-44d1-9392-a50c84570ada, 9ba58b6a-bc85-44a5-a720-d8e7a6ff7b31, 62e50382-db1b-4649-9f26-7f44c379d6fe, ce19c883-c61d-477b-a6d4-fee5134c6e9a, 946263aa-519a-47aa-b743-5cdcf52f5d35, 796a2544-43e5-43db-b5d3-8a4e737ec9c6, 6108b8b2-911e-443b-abe0-4c6f8b407384, bc4291c8-98f1-499d-8e02-46e371035e70, 2db33118-82b4-4edf-a9cb-e92ac62738b9
id: 4b407ee1-1e64-4ca1-842b-69773fc0e48a

Problem: depends is directional and two decisions have no new side; one reading gives one direction, two readings double the cost.
Standard: read a symmetric relation once; for a directional one, state the direction in the answer.

- A) No.
  Why: each pair is read once in one fixed order; the reader's sentence names which stands on which.
  Costs: the one directional word, depends, is recorded under the pair's first text only.
- B) Yes.
  Why: each decision has its own row for the pair, as in planning; direction is in the row, not the sentence.
  Costs: twice the reader calls, and two words that may disagree for one pair.

### Does a commit run the reading of the set?
depends: f90dbd5b-8fbe-4899-8588-f8762795c2e2, 1ab6d6c3-7e44-42de-bc2f-2e5849114aef=A
unrelated: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, 34e81f57-9b3c-4dc4-a2ba-ca227523177d, a26876ba-5789-4d05-965e-853d00d9edd2, bf6697e3-8832-4b90-b036-8fecedb55406, 26c5352f-3f82-4376-9dc2-8ccc0074d080, 19e1c023-a678-4842-a414-bb0e45c41601, 76c6adc6-c018-4df4-b49a-ddcd0a03a48c, dfd32ef6-c2ac-40ef-b8b3-9c58732516b1, 1c27f2d6-ab62-4c5a-91b2-1488722f3c48, d8a9dfff-de89-442c-a08c-8276efab3f36, 4f925b13-ad16-48b8-acd1-3e30d6e308b6, c3127af2-4c96-48d5-a164-bac49a9a09ec, f422c5db-13d0-4ff7-9e43-b68ab44a7cb9, db56a285-946f-48b1-b073-bb1e90192185, 908bddc6-5d30-4bcc-be8d-4c470c6bff11, 4b407ee1-1e64-4ca1-842b-69773fc0e48a, 2db33118-82b4-4edf-a9cb-e92ac62738b9, 4f04bae4-98e3-48eb-9403-3257268e2d87
id: bc4291c8-98f1-499d-8e02-46e371035e70

Problem: the reading needs the model and the reader and takes minutes; a commit runs on every machine, with neither, in seconds.
Standard: expensive advisory analyses run on demand, not in a commit hook.

- A) No.
  Why: the agent runs it at its own call, after a merge or a round of answers; hooks stay model-free.
  Costs: a set nobody reads stays unread; nothing reminds the agent.
- B) Yes.
  Why: the set is never stale; every commit reads what changed.
  Costs: every commit needs the model and the reader and waits minutes; a hook without them fails.

### Does the tool print every pair the reader marks?
depends: be0d6f5c-ae1b-4753-8ec7-0f4b03d84641, 1ab6d6c3-7e44-42de-bc2f-2e5849114aef=A, 908bddc6-5d30-4bcc-be8d-4c470c6bff11=*, 08a1b488-32fc-4dc7-b4b3-93a6e768725f
unrelated: 97c679f6-e8e6-4970-b547-940477753d7d, 75075c78-fe3c-4fd8-8a33-29a5913d9d34, 6108b8b2-911e-443b-abe0-4c6f8b407384, 385a57d5-5f76-454a-ae5d-9b0d6f5c1492, 902bd279-e6c1-4a21-9637-a3a43f360bae, 7b05f8df-09a5-404e-a0f0-3dd8824d1d15, 26c5352f-3f82-4376-9dc2-8ccc0074d080, 4f04bae4-98e3-48eb-9403-3257268e2d87, 154e4b9a-c811-4a84-a522-598914eb39f1, bf6697e3-8832-4b90-b036-8fecedb55406, 5892981c-f213-404f-b93f-551e4d081ebf, ce19c883-c61d-477b-a6d4-fee5134c6e9a, 055d303b-c286-4f6c-975a-d9d8090936d4, 4b407ee1-1e64-4ca1-842b-69773fc0e48a, bc4291c8-98f1-499d-8e02-46e371035e70
id: 2db33118-82b4-4edf-a9cb-e92ac62738b9

Problem: the rows go to a file nobody opens; a pair the reader marks, same, inconsistent or depends, has to reach the agent as a list it can act on.
Standard: a scan prints its findings and the move for each; the store is for the record.

- A) Yes.
  Why: one line per marked pair: both headings, the word, the reason; the move is a supersede question.
  Costs: a long list on a set with many ties; depends pairs are printed with no move.
- B) No.
  Why: the rows are the record; show prints them under each decision.
  Costs: the agent reads show for a hundred decisions to find three findings.
