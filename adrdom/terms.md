# Terms

## owner
id: 1d7e59ea-99e3-4d4b-9928-5dabca62723d
definition: The person the repository belongs to.
aliases: owners

## agent
id: f90dbd5b-8fbe-4899-8588-f8762795c2e2
terms: owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d
definition: A program that writes into the repository on the owner's behalf.
aliases: agents

## identifier
id: e877e4a9-2caf-4fe6-9f82-5476a2abb00b
definition: A unique value the tool assigns once and never reuses.
aliases: identifiers, uuid, uuids

## ledger
id: e18b82d8-5271-4385-bbab-2b89e5da278f
definition: The append-only file under adrdom/state with one row per event.
aliases:

## option
id: 43778946-d44c-4acf-a766-0a4915fce199
terms: owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d
definition: One lettered sentence under a heading that the owner may pick.
aliases: options

## question
id: 277a8338-c57a-43cc-8b28-6b42e9347f6a
terms: options=43778946-d44c-4acf-a766-0a4915fce199
definition: One heading ending in ? followed by two or more options.
aliases: questions

## decision
id: 41c41584-1ea3-4f15-b7ec-76a1dc99d32f
terms: option=43778946-d44c-4acf-a766-0a4915fce199, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d
definition: The option the owner picked.
aliases: decisions

## answer
id: b36113a1-1384-4e4c-a798-8399f4181366
terms: option=43778946-d44c-4acf-a766-0a4915fce199, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, refusal=0942e445-0163-423e-8ba7-84dd8ac07045, refusing=0942e445-0163-423e-8ba7-84dd8ac07045
definition: The owner's act of picking one option of a question or refusing it, and the line under the question that names the pick or the refusal.
aliases: answers, answered, answering

## ask
id: 3c8d4c9c-abca-4faa-b7c1-aae39951376c
terms: ledger=e18b82d8-5271-4385-bbab-2b89e5da278f, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a
definition: The act that puts a question to the owner and writes that event to the ledger.
aliases: asks, asked, asking

## current
id: e2533580-1486-4dc9-8697-e6c53605fe72
terms: answered=b36113a1-1384-4e4c-a798-8399f4181366, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, superseded=2bae531b-c4df-48a0-89a0-3d52726741ea
definition: The state of a decision that is answered and not superseded.
aliases:

## supersede
id: 2bae531b-c4df-48a0-89a0-3d52726741ea
terms: answered=b36113a1-1384-4e4c-a798-8399f4181366, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f
definition: A newer decision naming an older one, which it replaces once answered.
aliases: supersedes, superseded, superseding

## topic
id: 292a10b8-c287-4d71-b2bd-840da0a6de61
terms: questions=277a8338-c57a-43cc-8b28-6b42e9347f6a
definition: One subject that a set of questions shares, one per file.
aliases: topics

## topic file
id: 08193e75-dd89-4857-8e9f-b19e1a9e75e6
terms: decisions=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, topic=292a10b8-c287-4d71-b2bd-840da0a6de61
definition: A markdown file under adrdom/decisions holding one topic.
aliases: topic files

## tier
id: f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c
terms: asks=3c8d4c9c-abca-4faa-b7c1-aae39951376c, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, questions=277a8338-c57a-43cc-8b28-6b42e9347f6a, stands on=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6
definition: The questions the tool asks together, one above the highest question each of them stands on in its topic file.
aliases: tiers

## terms file
id: 528fe6f6-6d77-42d5-9086-017048d1448d
definition: The one file under adrdom that fixes the meaning of words.
aliases:

## approve
id: e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd
terms: owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, terms file=528fe6f6-6d77-42d5-9086-017048d1448d, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6
definition: The owner's act of accepting a topic file's headings and prose, or the terms file, as they stand.
aliases: approves, approved, approving, approval, approvals

## plan
id: 94677ed8-eaf6-46e2-bc71-7392fa0805bf
terms: approved=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, stands on=aef1b384-2b33-47a2-92a8-ec7f8fb258ef, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6
definition: The approved headings of a topic file and what each stands on.
aliases: plans

## withdraw
id: e4a2ef0c-3354-4c9b-a8d9-9e204e544d85
terms: approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, asked=3c8d4c9c-abca-4faa-b7c1-aae39951376c, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, question=277a8338-c57a-43cc-8b28-6b42e9347f6a
definition: The event approve writes for a question that was asked and then struck from the plan.
aliases: withdraws, withdrawn, withdrawing

## check
id: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de
terms: answered=b36113a1-1384-4e4c-a798-8399f4181366, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, refuses=0942e445-0163-423e-8ba7-84dd8ac07045
definition: The command that reads the repository against the ledger and refuses what breaks a rule the owner answered.
aliases: checks, checked, checking

## term
id: f590059d-b681-40ec-8a1c-c8916d71f6bb
terms: aliases=1929dc82-4128-4621-af0e-043ac181459f, definition=607cb55f-c0fa-40af-b290-c17f3ba0f812, identifier=e877e4a9-2caf-4fe6-9f82-5476a2abb00b, terms file=528fe6f6-6d77-42d5-9086-017048d1448d
definition: A word or phrase given one fixed meaning in the terms file, with an identifier, a definition, and aliases.
aliases: terms

## definition
id: 607cb55f-c0fa-40af-b290-c17f3ba0f812
definition: One or two sentences saying what a thing is, never what it does.
aliases: definitions

## alias
id: 1929dc82-4128-4621-af0e-043ac181459f
definition: Another word or phrase with the same fixed meaning as a name.
aliases: aliases

## retire
id: c584a68d-e54a-4ad2-8a6b-9d64ed098ae5
terms: term=f590059d-b681-40ec-8a1c-c8916d71f6bb
definition: Marking a term so nothing new may pair it while old lines still find it.
aliases: retires, retired, retiring

## ban
id: ef83beae-6857-4200-b251-a0fcc3c72f9d
definition: Adding the hash of a word to the list nothing in the repository may contain.
aliases: bans, banned, banning

## install
id: 1b98cdea-dc1b-4789-8f67-68d130d7a49d
terms: brief=1d9df88a-40f6-4235-a083-e063fde634fe
definition: The act that puts the tool's hooks and brief into a repository.
aliases: installs, installed, installing

## ui
id: 7127ab1e-c313-4c51-9134-c09d551682cf
terms: owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d
definition: The part of the tool the owner sees and uses in a browser.
aliases: user interface, web ui

## depends
id: aef1b384-2b33-47a2-92a8-ec7f8fb258ef
terms: current=e2533580-1486-4dc9-8697-e6c53605fe72, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, question=277a8338-c57a-43cc-8b28-6b42e9347f6a, term=f590059d-b681-40ec-8a1c-c8916d71f6bb, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6
definition: The line under a question heading naming what the question rests on: a term, a question in the same topic file, or a current decision.
aliases: stands on

## refusal
id: 0942e445-0163-423e-8ba7-84dd8ac07045
terms: asked=3c8d4c9c-abca-4faa-b7c1-aae39951376c, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, question=277a8338-c57a-43cc-8b28-6b42e9347f6a
definition: The owner's ruling that a question cannot be settled as asked.
aliases: refuse, refuses, refused, refusing

## answer set
id: 9d634996-475e-4ddc-8113-05c404ad9d5c
terms: asked=3c8d4c9c-abca-4faa-b7c1-aae39951376c, options=43778946-d44c-4acf-a766-0a4915fce199, question=277a8338-c57a-43cc-8b28-6b42e9347f6a
definition: The letters of a question's options under which a question standing on it is asked.
aliases: answer sets

## origin
id: ecb18413-cb3e-437e-9444-3cdd406ed67d
definition: A prose file outside adrdom whose text the records take in.
aliases: origins

## blame
id: 108053a8-c2c3-4561-874c-2f573a5409f2
terms: origin=ecb18413-cb3e-437e-9444-3cdd406ed67d
definition: A file holding, for one origin, its line ranges and what each range went to in the records.
aliases: blame file, blame files

## terms brief
id: 7d44030a-3589-4f29-aaa6-12fa09908ab8
terms: aliases=1929dc82-4128-4621-af0e-043ac181459f, definition=607cb55f-c0fa-40af-b290-c17f3ba0f812, retired=c584a68d-e54a-4ad2-8a6b-9d64ed098ae5, term=f590059d-b681-40ec-8a1c-c8916d71f6bb
definition: The file under adrdom/state holding the name, aliases and definition of every term that is not retired, one term per line.
aliases: terms briefs

## brief
id: 1d9df88a-40f6-4235-a083-e063fde634fe
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2
definition: The file at the root of a repository that tells an agent how to work in it.
aliases: briefs

## embedding
id: fa0b0e24-b7cf-471b-94b6-6d167e8ad269
definition: A list of numbers the tool computes from a text, so that two texts on one subject give two near lists.
aliases: embeddings

## embedding model
id: a7b0b910-57f6-4c59-b904-b42f78db63f7
terms: embedding=fa0b0e24-b7cf-471b-94b6-6d167e8ad269
definition: The file the tool reads to compute an embedding.
aliases: embedding models
