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

### Does the word limit hold for the Why line of an option?
depends: 43778946-d44c-4acf-a766-0a4915fce199, d808917b-0717-45ad-ab83-9fa274b9a9d4=A, 055d303b-c286-4f6c-975a-d9d8090936d4
id: 39b76886-98f9-4019-9194-db9ab51aea1e

### Does the word limit hold for the Costs line of an option?
depends: 43778946-d44c-4acf-a766-0a4915fce199, d808917b-0717-45ad-ab83-9fa274b9a9d4=A, 055d303b-c286-4f6c-975a-d9d8090936d4
id: 32a189a0-86e0-44d1-9392-a50c84570ada

### Does the word limit read answered text?
depends: b36113a1-1384-4e4c-a798-8399f4181366, d808917b-0717-45ad-ab83-9fa274b9a9d4=A, 923818a7-05c6-4977-bbaf-53ee8f3773ef
id: 423065fc-6d9f-40c8-a0bf-8e1779e86b38
