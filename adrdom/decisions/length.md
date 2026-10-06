# Length

How long the lines of a question may be. An option that holds more than one rule lets a
rule the owner never gave pass under one letter; a short line holds one.

### Does an option under a heading that opens with Does hold one word?
depends: 43778946-d44c-4acf-a766-0a4915fce199, 277a8338-c57a-43cc-8b28-6b42e9347f6a
id: b3cb190d-d6f1-4930-bdcd-7827e31ac62a
terms: answer=b36113a1-1384-4e4c-a798-8399f4181366, approves=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, asked=3c8d4c9c-abca-4faa-b7c1-aae39951376c, option=43778946-d44c-4acf-a766-0a4915fce199, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, refuses=0942e445-0163-423e-8ba7-84dd8ac07045

Problem: an option that opens with Yes and goes on carries rules the heading never asked, and one letter approves them all.
Standard: a closed question takes a closed answer; a yes/no item in a form has no free text under the choice.

- A) Yes.
  Why: the rule lives in the heading, where the tool refuses "and", "or" and a loaded opening.
  Costs: a rule with a condition needs a second heading, and a plan grows by the count of rules.
- B) No.
  Why: an option can state the rule in full under the heading.
  Costs: one letter carries every clause of the sentence, read or not.

**Answer:** A

### Does ask refuse an option over a word limit?
depends: 3c8d4c9c-abca-4faa-b7c1-aae39951376c, 43778946-d44c-4acf-a766-0a4915fce199
id: d808917b-0717-45ad-ab83-9fa274b9a9d4
terms: ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, checked=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, option=43778946-d44c-4acf-a766-0a4915fce199, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, refuse=0942e445-0163-423e-8ba7-84dd8ac07045

Problem: a long option holds several rules, and the owner picks one letter and gives them all.
Standard: style linters cap sentence length and fail the build on a line over the cap.

- A) Yes.
  Why: the limit is a count, the same in every repository, checked before the owner reads.
  Costs: a rule that needs more words becomes two questions.
- B) No.
  Why: nothing is added to the tool, and the owner reads the length.
  Costs: the owner is the only guard, and a long option is where a rule hides.

**Answer:** A

### What is the word limit of an option?
depends: 43778946-d44c-4acf-a766-0a4915fce199, d808917b-0717-45ad-ab83-9fa274b9a9d4=A
id: 8f324216-80a3-4012-b689-e27b6eea7714
terms: option=43778946-d44c-4acf-a766-0a4915fce199, refused=0942e445-0163-423e-8ba7-84dd8ac07045

Problem: a limit too low cuts a one-rule sentence, and a limit too high lets two rules through.
Standard: plain-language guides cap a sentence at twenty to twenty-five words, and readability tools flag longer ones.

- A) Twenty.
  Why: a rule with its condition fits in twenty words, and a third rule does not.
  Costs: two short rules fit in twenty words.
- B) Fifteen.
  Why: two rules seldom fit in fifteen words.
  Costs: a one-rule sentence with a condition is refused and split.
- C) Twenty-five.
  Why: room for a rule, its condition and an example.
  Costs: three rules fit in twenty-five words.

**Answer:** A

### Does the word limit hold for the Why line of an option?
depends: 43778946-d44c-4acf-a766-0a4915fce199, d808917b-0717-45ad-ab83-9fa274b9a9d4=A, 055d303b-c286-4f6c-975a-d9d8090936d4
id: 39b76886-98f9-4019-9194-db9ab51aea1e
terms: checks=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, option=43778946-d44c-4acf-a766-0a4915fce199

Problem: a Why line that runs on argues for its letter with claims nobody checks.
Standard: a rationale line in a decision record is one sentence.

- A) Yes.
  Why: one number for every line, one count in the tool.
  Costs: a gain with two parts is cut to one.
- B) No.
  Why: the Why line binds nothing, and its length harms no one.
  Costs: the long line comes back under Why.

**Answer:** A

### Does the word limit hold for the Costs line of an option?
depends: 43778946-d44c-4acf-a766-0a4915fce199, d808917b-0717-45ad-ab83-9fa274b9a9d4=A, 055d303b-c286-4f6c-975a-d9d8090936d4
id: 32a189a0-86e0-44d1-9392-a50c84570ada
terms: decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, option=43778946-d44c-4acf-a766-0a4915fce199

Problem: a Costs line that states mechanics is a rule by another name, and a builder reads it as one.
Standard: a consequences line in a decision record is one sentence.

- A) Yes.
  Why: a second rule has no line left to hide in.
  Costs: a cost with its consequence is cut to the cost.
- B) No.
  Why: the Costs line needs room to say what the code has to keep.
  Costs: the rule moves from the option to its Costs line.

**Answer:** A

### Does the word limit read answered text?
depends: b36113a1-1384-4e4c-a798-8399f4181366, d808917b-0717-45ad-ab83-9fa274b9a9d4=A, 923818a7-05c6-4977-bbaf-53ee8f3773ef
id: 423065fc-6d9f-40c8-a0bf-8e1779e86b38
terms: answer=b36113a1-1384-4e4c-a798-8399f4181366, answered=b36113a1-1384-4e4c-a798-8399f4181366, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, asks=3c8d4c9c-abca-4faa-b7c1-aae39951376c, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, option=43778946-d44c-4acf-a766-0a4915fce199, options=43778946-d44c-4acf-a766-0a4915fce199, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, supersede=2bae531b-c4df-48a0-89a0-3d52726741ea, superseded=2bae531b-c4df-48a0-89a0-3d52726741ea

Problem: a limit that reads answered options fails every decision written before it, and only a supersede of each clears them.
Standard: a new lint rule runs on changed lines, and old text is left alone.

- A) No.
  Why: ask reads the limit on the question it asks, and an answered option is frozen with its answer.
  Costs: long answered options are left alone.
- B) Yes.
  Why: one shape for every option in the records.
  Costs: every decision over the limit is superseded before check passes.

**Answer:** A

### Does the word limit hold for an option's Why line?
depends: 43778946-d44c-4acf-a766-0a4915fce199, d808917b-0717-45ad-ab83-9fa274b9a9d4, 178a232e-b51c-4122-a1a9-9e9ed6970748, 8f324216-80a3-4012-b689-e27b6eea7714
unrelated: b3cb190d-d6f1-4930-bdcd-7827e31ac62a, 28b5837d-46ae-459e-ba4c-12265ef07ac1, 277a8338-c57a-43cc-8b28-6b42e9347f6a, a82db5b7-969c-44e9-9eec-876ead7099d3, b36113a1-1384-4e4c-a798-8399f4181366, 902bd279-e6c1-4a21-9637-a3a43f360bae, 9d634996-475e-4ddc-8113-05c404ad9d5c, 05af74a6-36be-4923-b66f-1467b323816a, 796a2544-43e5-43db-b5d3-8a4e737ec9c6, 5c399a14-1e5c-48eb-b502-b57524c01c24, ad9761e2-6d64-4b45-b0ba-5c5d8d2b38bf, 908bddc6-5d30-4bcc-be8d-4c470c6bff11, c96518bf-470d-45c0-8b86-eaf8f54cb490, 55a8abb2-0f6d-430b-99f9-a378f3f154e5
id: 0487a1cf-eb61-4bf8-bd16-eed899d22d2c
terms: checks=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, option=43778946-d44c-4acf-a766-0a4915fce199

Problem: a Why line that runs on argues for its letter with claims nobody checks.
Standard: a rationale line in a decision record is one sentence.

- A) Yes.
  Why: one number for every line, one count in the tool.
  Costs: a gain with two parts is cut to one.
- B) No.
  Why: the Why line binds nothing, and its length harms no one.
  Costs: the long line comes back under Why.

### Does the word limit hold for an option's Costs line?
depends: 43778946-d44c-4acf-a766-0a4915fce199, d808917b-0717-45ad-ab83-9fa274b9a9d4, 178a232e-b51c-4122-a1a9-9e9ed6970748, 8f324216-80a3-4012-b689-e27b6eea7714
unrelated: b3cb190d-d6f1-4930-bdcd-7827e31ac62a, a82db5b7-969c-44e9-9eec-876ead7099d3, 28b5837d-46ae-459e-ba4c-12265ef07ac1, 05af74a6-36be-4923-b66f-1467b323816a, 796a2544-43e5-43db-b5d3-8a4e737ec9c6, b36113a1-1384-4e4c-a798-8399f4181366, f1294391-48b0-4ba3-82b4-c1d6b1c2c884, c584a68d-e54a-4ad2-8a6b-9d64ed098ae5, c3127af2-4c96-48d5-a164-bac49a9a09ec, aef1b384-2b33-47a2-92a8-ec7f8fb258ef, ad9761e2-6d64-4b45-b0ba-5c5d8d2b38bf, f4f667ad-8714-4e65-897b-c7002e423c28, 0487a1cf-eb61-4bf8-bd16-eed899d22d2c, 55a8abb2-0f6d-430b-99f9-a378f3f154e5
id: c96518bf-470d-45c0-8b86-eaf8f54cb490
terms: decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, option=43778946-d44c-4acf-a766-0a4915fce199

Problem: a Costs line that states mechanics is a rule by another name, and a builder reads it as one.
Standard: a consequences line in a decision record is one sentence.

- A) Yes.
  Why: a second rule has no line left to hide in.
  Costs: a cost with its consequence is cut to the cost.
- B) No.
  Why: the Costs line needs room to say what the code has to keep.
  Costs: the rule moves from the option to its Costs line.

### Does the word limit read the text of an answered question?
depends: b36113a1-1384-4e4c-a798-8399f4181366, d808917b-0717-45ad-ab83-9fa274b9a9d4, 149c9a73-16eb-46aa-9806-2950423ee1a2, 8f324216-80a3-4012-b689-e27b6eea7714, cd83e277-ebf0-4458-a77a-c526ab4cfeca
unrelated: 6108b8b2-911e-443b-abe0-4c6f8b407384, 902bd279-e6c1-4a21-9637-a3a43f360bae, 11be073c-ce91-44d8-833a-ef0b5e0b1672, 97c679f6-e8e6-4970-b547-940477753d7d, 946263aa-519a-47aa-b743-5cdcf52f5d35, 735a32bd-db0c-428e-b65e-2acc750671b0, f571a454-3e6c-46c1-b754-0822a5d1d667, e3200081-9995-4bbe-983f-2f430dc341e7, b782664f-7465-4e1e-99e0-13f6dd2de774, da59b448-1afb-4469-b40e-69eda8f253c2, 711a7636-9f4d-402d-ae62-265208adf0ac, db58d4b8-9e64-44b0-99a2-acb7f05d5b4e, c96518bf-470d-45c0-8b86-eaf8f54cb490, 0487a1cf-eb61-4bf8-bd16-eed899d22d2c
id: 55a8abb2-0f6d-430b-99f9-a378f3f154e5
terms: answer=b36113a1-1384-4e4c-a798-8399f4181366, answered=b36113a1-1384-4e4c-a798-8399f4181366, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, asks=3c8d4c9c-abca-4faa-b7c1-aae39951376c, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, option=43778946-d44c-4acf-a766-0a4915fce199, options=43778946-d44c-4acf-a766-0a4915fce199, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, supersede=2bae531b-c4df-48a0-89a0-3d52726741ea, superseded=2bae531b-c4df-48a0-89a0-3d52726741ea

Problem: a limit that reads answered options fails every decision written before it, and only a supersede of each clears them.
Standard: a new lint rule runs on changed lines, and old text is left alone.

- A) No.
  Why: ask reads the limit on the question it asks, and an answered option is frozen with its answer.
  Costs: long answered options are left alone.
- B) Yes.
  Why: one shape for every option in the records.
  Costs: every decision over the limit is superseded before check passes.
