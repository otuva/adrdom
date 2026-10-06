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
depends: option, Does ask refuse an option over a word limit?, What stands under a question's heading?, What is the word limit of an option?
unrelated: Does an option under a heading that opens with Does hold one word?, What binds in an answered question?, question, Does a reference line hold its list on one line?, answer, Does the reader's word stop a command?, answer set, Does a retired term keep its words?, Does a supersedes line name every current decision standing on its target?, Does a blame file cover its whole origin?, What does a depends line name?, What does the reader say about two current decisions?, Does the word limit hold for an option's Costs line?, Does the word limit read the text of an answered question?

Problem: a Why line that runs on argues for its letter with claims nobody checks.
Standard: a rationale line in a decision record is one sentence.

- A) Yes.
  Why: one number for every line, one count in the tool.
  Costs: a gain with two parts is cut to one.
- B) No.
  Why: the Why line binds nothing, and its length harms no one.
  Costs: the long line comes back under Why.

### Does the word limit hold for an option's Costs line?
depends: option, Does ask refuse an option over a word limit?, What stands under a question's heading?, What is the word limit of an option?
unrelated: Does an option under a heading that opens with Does hold one word?, Does a reference line hold its list on one line?, What binds in an answered question?, Does a retired term keep its words?, Does a supersedes line name every current decision standing on its target?, answer, What does the terms brief hold?, retire, Does the agent build what no current decision covers?, depends, What does a depends line name?, Does one decision supersede several?, Does the word limit hold for an option's Why line?, Does the word limit read the text of an answered question?

Problem: a Costs line that states mechanics is a rule by another name, and a builder reads it as one.
Standard: a consequences line in a decision record is one sentence.

- A) Yes.
  Why: a second rule has no line left to hide in.
  Costs: a cost with its consequence is cut to the cost.
- B) No.
  Why: the Costs line needs room to say what the code has to keep.
  Costs: the rule moves from the option to its Costs line.

### Does the word limit read the text of an answered question?
depends: answer, Does ask refuse an option over a word limit?, What holds the list of loaded phrases?, What is the word limit of an option?, Does check fail an answered question whose text changed?
unrelated: Does check read the words of an answered question again?, Does the reader's word stop a command?, Does an answer carry the version of the question the owner read?, Does show print the neighbours of a question with the reader's word?, Does a depends entry on a question in its file carry an answer set?, Does a banned word pass when it is only quoted?, Does ask pair the whole question to terms?, Does check fail a question answered with two different options?, Does the owner answer a question with a refusal?, Does ask stop a question outside the plan?, Does ask refuse a question with no term?, Does the owner retire a term that a question names?, Does the word limit hold for an option's Costs line?, Does the word limit hold for an option's Why line?

Problem: a limit that reads answered options fails every decision written before it, and only a supersede of each clears them.
Standard: a new lint rule runs on changed lines, and old text is left alone.

- A) No.
  Why: ask reads the limit on the question it asks, and an answered option is frozen with its answer.
  Costs: long answered options are left alone.
- B) Yes.
  Why: one shape for every option in the records.
  Costs: every decision over the limit is superseded before check passes.
