# Satellite

One record may govern more than one repository: the primary holds the record, a satellite holds code and tests and no record of its own. These questions settle what install, check, show and uninstall do in each.

### Does install take the path of a satellite?
depends: 1b98cdea-dc1b-4789-8f67-68d130d7a49d, dd50488e-a8cd-4778-aecc-72295ffd8671, c6fc0041-dead-4e31-9c19-8913c1bdfa29
unrelated: 459c1b77-82dd-469f-a60c-784b6f3d7b81, e1c305af-af94-494a-9fdb-3faffafa0e06, aac5c313-dedd-466b-a0c3-ead5e99ce913, 6ef9eca6-fd8b-4211-aae7-00b37a2a9245, 19e1c023-a678-4842-a414-bb0e45c41601, 4f925b13-ad16-48b8-acd1-3e30d6e308b6, 292359a4-0512-465b-b6a7-79838e66e5a5, c304b7da-1843-43ad-930c-b01b3310483d, 5418b256-dfae-439a-96c1-57d82380cd1e, fc118154-4172-48db-bddb-75a691894edf, a8280dd1-4bfa-4eeb-9656-1cd06381590a, 7e756e04-dde0-4137-ad4f-646e52073398, dfd32ef6-c2ac-40ef-b8b3-9c58732516b1, 76c6adc6-c018-4df4-b49a-ddcd0a03a48c, 99619e27-19b6-47ef-83b9-c79ac30ee2f0, db56a285-946f-48b1-b073-bb1e90192185, ae63df95-0643-4108-af65-1e7a0009d631, 0e16fbf3-1401-4538-bd12-25303994af10, 03f47941-43c5-479f-bf64-2d030361092d, 563ba307-4c7d-4945-9c89-46dfa61cb88d, 13ddec51-ce03-4645-99ae-2bbf4f30712d, 30063002-420c-4309-bae2-4175b993f8f1, da777c6e-f22a-41fa-ba6f-1f1544382710, cb15b115-610f-4c82-a8f7-6bae9c131dc0
id: 937bc0a6-691d-4d67-91c6-cc9e956ebc96
terms: install=1b98cdea-dc1b-4789-8f67-68d130d7a49d, primary=c6fc0041-dead-4e31-9c19-8913c1bdfa29, record=68536615-d17e-4399-ad6c-2a2cf8622b2b, satellite=dd50488e-a8cd-4778-aecc-72295ffd8671

Problem: nothing makes a repository a satellite of a record, so a second repository has to carry a record of its own.
Standard: a hub-and-spoke tool learns its spokes at the hub, once, by one command.

- A) Yes.
  Why: one command in the primary names the satellite, writes both sides and leaves nothing to type by hand.
  Costs: the satellite's checkout has to be on disk when the command runs.
- B) No.
  Why: install keeps one shape; a satellite is made by hand.
  Costs: two files written by hand that have to agree, with nothing to say when they do not.

### What does check in a satellite read the record from?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, 68536615-d17e-4399-ad6c-2a2cf8622b2b, dd50488e-a8cd-4778-aecc-72295ffd8671, c6fc0041-dead-4e31-9c19-8913c1bdfa29
unrelated: 34e81f57-9b3c-4dc4-a2ba-ca227523177d, 0c3d498a-c540-47f3-8384-050fc044c109, 6108b8b2-911e-443b-abe0-4c6f8b407384, d904dc0d-231f-4e93-ac74-7babe888107d, bd83aa22-94fc-49ac-b215-a82990ff8c65, 48c47c42-6720-44ca-b47d-12a61ba8ee9f, 3460d220-e523-448b-aeff-baa8051b7d06, c10c3a49-e993-4c9f-a46b-4a1ba3646dcf, 80b89c24-7d65-43f4-bb03-55e9d85c327d, fc118154-4172-48db-bddb-75a691894edf, a8280dd1-4bfa-4eeb-9656-1cd06381590a, 7e756e04-dde0-4137-ad4f-646e52073398, bf6697e3-8832-4b90-b036-8fecedb55406, ce19c883-c61d-477b-a6d4-fee5134c6e9a, 19e1c023-a678-4842-a414-bb0e45c41601, 1af76f91-5644-42ef-97ea-8f70f934c26c, 05141f49-d9d2-407c-b60c-ebb195a05864, 0e16fbf3-1401-4538-bd12-25303994af10, 03f47941-43c5-479f-bf64-2d030361092d, 563ba307-4c7d-4945-9c89-46dfa61cb88d, 13ddec51-ce03-4645-99ae-2bbf4f30712d, 937bc0a6-691d-4d67-91c6-cc9e956ebc96, 30063002-420c-4309-bae2-4175b993f8f1, da777c6e-f22a-41fa-ba6f-1f1544382710
id: cb15b115-610f-4c82-a8f7-6bae9c131dc0
terms: answer=b36113a1-1384-4e4c-a798-8399f4181366, ban=ef83beae-6857-4200-b251-a0fcc3c72f9d, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, install=1b98cdea-dc1b-4789-8f67-68d130d7a49d, ledger=e18b82d8-5271-4385-bbab-2b89e5da278f, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, primary=c6fc0041-dead-4e31-9c19-8913c1bdfa29, record=68536615-d17e-4399-ad6c-2a2cf8622b2b, records=68536615-d17e-4399-ad6c-2a2cf8622b2b, satellite=dd50488e-a8cd-4778-aecc-72295ffd8671, terms file=528fe6f6-6d77-42d5-9086-017048d1448d, topic file=08193e75-dd89-4857-8e9f-b19e1a9e75e6

Problem: a satellite has no ledger, no terms file and no topic file; check in it has no record to judge against.
Standard: a tool with a remote configuration reads it from the one place named, and fails loudly when it is missing.

- A) The primary's working tree, through the path the satellite holds; no record there and check stops.
  Why: one reading of the record for every command, the one the owner sees on disk; nothing is guessed.
  Costs: the primary's checkout has to sit at the path on every machine, CI included.
- B) The primary's last commit, through the path the satellite holds; no record there and check stops.
  Why: a satellite is judged only against a record that is in the primary's history.
  Costs: a ban or an answer not yet committed in the primary binds nothing in a satellite.
- C) A copy of the record install writes into the satellite.
  Why: a satellite stands alone, with no checkout beside it.
  Costs: every answer in the primary waits for a copy, and two records drift.

### What does the primary hold of a satellite?
depends: c6fc0041-dead-4e31-9c19-8913c1bdfa29, dd50488e-a8cd-4778-aecc-72295ffd8671
unrelated: 209081f4-864f-4ef6-8909-65da87f135e5, 1d7e59ea-99e3-4d4b-9928-5dabca62723d, 7c306235-6243-4d79-b9aa-08ed19662779, aac5c313-dedd-466b-a0c3-ead5e99ce913, 946263aa-519a-47aa-b743-5cdcf52f5d35, f53ac3b9-efde-427c-8027-19988aa8fda7, 24bdcbde-d9f4-45a0-8c3d-37e53569e950, b0554224-15e4-4e91-a2bb-9ae5667803f4, 149c9a73-16eb-46aa-9806-2950423ee1a2, 9afe335c-2a7a-4625-b115-f74d3f091cac, cb15b115-610f-4c82-a8f7-6bae9c131dc0, fc118154-4172-48db-bddb-75a691894edf, a8280dd1-4bfa-4eeb-9656-1cd06381590a, ce19c883-c61d-477b-a6d4-fee5134c6e9a, 76c6adc6-c018-4df4-b49a-ddcd0a03a48c, 99619e27-19b6-47ef-83b9-c79ac30ee2f0, 16a71892-b348-4b9b-82c5-a9d375187bd3, 50b31713-4fea-4aa3-842a-79c2750660ba, 4319a653-6901-446c-94ab-6420ef472134, a469ecb9-a4f1-4a2a-89c5-7378cc1138b4, 55a6a2d8-03af-4db9-a4c7-e2790e38357e, 0e16fbf3-1401-4538-bd12-25303994af10, 03f47941-43c5-479f-bf64-2d030361092d, 563ba307-4c7d-4945-9c89-46dfa61cb88d, 13ddec51-ce03-4645-99ae-2bbf4f30712d, 937bc0a6-691d-4d67-91c6-cc9e956ebc96, 30063002-420c-4309-bae2-4175b993f8f1, da777c6e-f22a-41fa-ba6f-1f1544382710
id: 7e756e04-dde0-4137-ad4f-646e52073398
terms: check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, moves=d904dc0d-231f-4e93-ac74-7babe888107d, primary=c6fc0041-dead-4e31-9c19-8913c1bdfa29, record=68536615-d17e-4399-ad6c-2a2cf8622b2b, records=68536615-d17e-4399-ad6c-2a2cf8622b2b, satellite=dd50488e-a8cd-4778-aecc-72295ffd8671, satellites=dd50488e-a8cd-4778-aecc-72295ffd8671

Problem: the primary cannot find the tests of a satellite it does not know.
Standard: a hub keeps the list of its spokes beside its own state, one row each.

- A) One tracked row under adrdom/state: the satellite's relative path and its remote url.
  Why: check finds the satellite by the path, and CI can clone it by the url.
  Costs: a satellite that moves or changes remote needs its row written again.
- B) One tracked row under adrdom/state: the satellite's relative path.
  Why: the tool records only what it reads itself.
  Costs: CI learns where each satellite comes from outside the record.
- C) Nothing; CI names the satellites.
  Why: the smallest tool.
  Costs: the primary's check cannot count a satellite's ties.

### Does a command that writes the record run in a satellite?
depends: dd50488e-a8cd-4778-aecc-72295ffd8671, 68536615-d17e-4399-ad6c-2a2cf8622b2b, cb15b115-610f-4c82-a8f7-6bae9c131dc0=*
unrelated: aac5c313-dedd-466b-a0c3-ead5e99ce913, aefa7f3c-70c3-412e-8d73-16ff9e2b703a, f53ac3b9-efde-427c-8027-19988aa8fda7, 1af76f91-5644-42ef-97ea-8f70f934c26c, 8846f6d1-cd8c-4b87-b6f2-35bc8adf6763, a9e9e370-7768-4a79-b67b-7fe0926b5670, e78b8297-fd89-4e09-9c3e-9eeb0fd0f654, 80b89c24-7d65-43f4-bb03-55e9d85c327d, dfd32ef6-c2ac-40ef-b8b3-9c58732516b1, 8081c82d-e5c6-485b-a302-07d586f1a01b, fc118154-4172-48db-bddb-75a691894edf, a8280dd1-4bfa-4eeb-9656-1cd06381590a, 7e756e04-dde0-4137-ad4f-646e52073398, b0e865b4-9204-4c55-9f5b-dcb755b3a03b, 4f04bae4-98e3-48eb-9403-3257268e2d87, f422c5db-13d0-4ff7-9e43-b68ab44a7cb9, bf27b020-4e60-48e0-8d3b-5f818e6bf6d8, 03f47941-43c5-479f-bf64-2d030361092d, 563ba307-4c7d-4945-9c89-46dfa61cb88d, 13ddec51-ce03-4645-99ae-2bbf4f30712d, 937bc0a6-691d-4d67-91c6-cc9e956ebc96, 30063002-420c-4309-bae2-4175b993f8f1, da777c6e-f22a-41fa-ba6f-1f1544382710
id: 0e16fbf3-1401-4538-bd12-25303994af10
terms: answer=b36113a1-1384-4e4c-a798-8399f4181366, approve=e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, ask=3c8d4c9c-abca-4faa-b7c1-aae39951376c, ban=ef83beae-6857-4200-b251-a0fcc3c72f9d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, plan=94677ed8-eaf6-46e2-bc71-7392fa0805bf, planner=3aedf314-f25f-49cb-b7b3-d2302e4543ca, primary=c6fc0041-dead-4e31-9c19-8913c1bdfa29, record=68536615-d17e-4399-ad6c-2a2cf8622b2b, refusal=0942e445-0163-423e-8ba7-84dd8ac07045, refuses=0942e445-0163-423e-8ba7-84dd8ac07045, satellite=dd50488e-a8cd-4778-aecc-72295ffd8671

Problem: plan, ask, approve, answer, ban, unban and serve write the record; in a satellite the record is in another repository.
Standard: a command that would write another repository's state refuses and names where to run.

- A) No.
  Why: the record is written in one place; the refusal names the primary's path.
  Costs: the owner and the planner change directory for every writing command.
- B) Yes.
  Why: the command writes through to the primary from any directory.
  Costs: a commit in the primary made from a satellite, by a hook that runs in the wrong repository.

### What does install write in a satellite?
depends: 1b98cdea-dc1b-4789-8f67-68d130d7a49d, dd50488e-a8cd-4778-aecc-72295ffd8671, 937bc0a6-691d-4d67-91c6-cc9e956ebc96=A
unrelated: ae63df95-0643-4108-af65-1e7a0009d631, 6ef9eca6-fd8b-4211-aae7-00b37a2a9245, c304b7da-1843-43ad-930c-b01b3310483d, 5418b256-dfae-439a-96c1-57d82380cd1e, db56a285-946f-48b1-b073-bb1e90192185, 459c1b77-82dd-469f-a60c-784b6f3d7b81, 292359a4-0512-465b-b6a7-79838e66e5a5, 19e1c023-a678-4842-a414-bb0e45c41601, e1c305af-af94-494a-9fdb-3faffafa0e06, 76c6adc6-c018-4df4-b49a-ddcd0a03a48c, fc118154-4172-48db-bddb-75a691894edf, 7e756e04-dde0-4137-ad4f-646e52073398, 4f925b13-ad16-48b8-acd1-3e30d6e308b6, c3127af2-4c96-48d5-a164-bac49a9a09ec, 23d53545-c238-48cc-ae5e-21a2c7aab83e, ec0b9293-916e-4ced-a3d7-3b862ab7b069, 0e16fbf3-1401-4538-bd12-25303994af10, 03f47941-43c5-479f-bf64-2d030361092d, 563ba307-4c7d-4945-9c89-46dfa61cb88d, 13ddec51-ce03-4645-99ae-2bbf4f30712d, 30063002-420c-4309-bae2-4175b993f8f1, da777c6e-f22a-41fa-ba6f-1f1544382710, cb15b115-610f-4c82-a8f7-6bae9c131dc0
id: a8280dd1-4bfa-4eeb-9656-1cd06381590a
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, agents=f90dbd5b-8fbe-4899-8588-f8762795c2e2, brief=1d9df88a-40f6-4235-a083-e063fde634fe, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, install=1b98cdea-dc1b-4789-8f67-68d130d7a49d, primary=c6fc0041-dead-4e31-9c19-8913c1bdfa29, satellite=dd50488e-a8cd-4778-aecc-72295ffd8671, terms brief=7d44030a-3589-4f29-aaa6-12fa09908ab8

Problem: an agent opening a satellite finds no brief and no hook, and writes code no decision covers.
Standard: every repository an agent works in carries the same brief and the same hooks.

- A) The primary's path under adrdom, copies of the brief and terms brief, the hooks and the AGENTS.md section.
  Why: the agent reads the same brief everywhere; the hook runs check at every commit.
  Costs: two copies to keep equal to the primary's.
- B) The primary's path under adrdom and the hooks; the brief is read in the primary.
  Why: nothing to keep equal.
  Costs: an agent that opens the satellite alone reads no brief.

### Does check in the primary count a tie in a satellite?
depends: c6fc0041-dead-4e31-9c19-8913c1bdfa29, dd50488e-a8cd-4778-aecc-72295ffd8671, 7e756e04-dde0-4137-ad4f-646e52073398=A|B
unrelated: 3460d220-e523-448b-aeff-baa8051b7d06, db07a54d-4d80-4035-a9ef-eabcc0c872b3, f53ac3b9-efde-427c-8027-19988aa8fda7, 24bdcbde-d9f4-45a0-8c3d-37e53569e950, 80b89c24-7d65-43f4-bb03-55e9d85c327d, 48c47c42-6720-44ca-b47d-12a61ba8ee9f, a26876ba-5789-4d05-965e-853d00d9edd2, 19e1c023-a678-4842-a414-bb0e45c41601, ce19c883-c61d-477b-a6d4-fee5134c6e9a, aac5c313-dedd-466b-a0c3-ead5e99ce913, 86d5e944-ecfa-44bc-9819-83ee4b96890d, fc118154-4172-48db-bddb-75a691894edf, a8280dd1-4bfa-4eeb-9656-1cd06381590a, 34e81f57-9b3c-4dc4-a2ba-ca227523177d, bd83aa22-94fc-49ac-b215-a82990ff8c65, 0c3d498a-c540-47f3-8384-050fc044c109, a469ecb9-a4f1-4a2a-89c5-7378cc1138b4, 05141f49-d9d2-407c-b60c-ebb195a05864, 2381d759-f85f-484e-9031-fadbe6ea045a, 0e16fbf3-1401-4538-bd12-25303994af10, 03f47941-43c5-479f-bf64-2d030361092d, 563ba307-4c7d-4945-9c89-46dfa61cb88d, 937bc0a6-691d-4d67-91c6-cc9e956ebc96, 30063002-420c-4309-bae2-4175b993f8f1, da777c6e-f22a-41fa-ba6f-1f1544382710, cb15b115-610f-4c82-a8f7-6bae9c131dc0
id: 13ddec51-ce03-4645-99ae-2bbf4f30712d
terms: check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, move=d904dc0d-231f-4e93-ac74-7babe888107d, primary=c6fc0041-dead-4e31-9c19-8913c1bdfa29, record=68536615-d17e-4399-ad6c-2a2cf8622b2b, satellite=dd50488e-a8cd-4778-aecc-72295ffd8671

Problem: a decision built in a satellite stays untested in the primary's eyes.
Standard: a traceability tool counts a tag wherever it scans, and scans every tree on its list.

- A) Yes.
  Why: one coverage for the whole record, and the build move names the satellite's path.
  Costs: every satellite's checkout at its path, or the count is short.
- B) No.
  Why: check reads one repository.
  Costs: a decision built in a satellite is untested for ever.

### Does show in a satellite name its primary?
depends: c6fc0041-dead-4e31-9c19-8913c1bdfa29, dd50488e-a8cd-4778-aecc-72295ffd8671, cb15b115-610f-4c82-a8f7-6bae9c131dc0=A|B
unrelated: c5e10ae8-258d-4f29-b9fe-13cb7ad2a550, f53ac3b9-efde-427c-8027-19988aa8fda7, d1d6fed2-4e1a-44bd-b9cb-22f779a00bce, 9afe335c-2a7a-4625-b115-f74d3f091cac, 796a2544-43e5-43db-b5d3-8a4e737ec9c6, b0e865b4-9204-4c55-9f5b-dcb755b3a03b, 24bdcbde-d9f4-45a0-8c3d-37e53569e950, a9e9e370-7768-4a79-b67b-7fe0926b5670, aac5c313-dedd-466b-a0c3-ead5e99ce913, 2bae531b-c4df-48a0-89a0-3d52726741ea, f4f667ad-8714-4e65-897b-c7002e423c28, fc118154-4172-48db-bddb-75a691894edf, 7e756e04-dde0-4137-ad4f-646e52073398, a8280dd1-4bfa-4eeb-9656-1cd06381590a, 11be073c-ce91-44d8-833a-ef0b5e0b1672, ec3f875c-a0f8-4085-8da1-fec9dcb14377, c3127af2-4c96-48d5-a164-bac49a9a09ec, adee66f2-c8ff-477f-8ac4-a497f694c912, ec0b9293-916e-4ced-a3d7-3b862ab7b069, 7c0b9fe3-6038-4a23-93fe-88a31c7eaf40, 9a2ff21b-c20b-4c63-95f2-a7a3cbe236b8, 0e16fbf3-1401-4538-bd12-25303994af10, 03f47941-43c5-479f-bf64-2d030361092d, 563ba307-4c7d-4945-9c89-46dfa61cb88d, 13ddec51-ce03-4645-99ae-2bbf4f30712d, 937bc0a6-691d-4d67-91c6-cc9e956ebc96, da777c6e-f22a-41fa-ba6f-1f1544382710, cb15b115-610f-4c82-a8f7-6bae9c131dc0
id: 30063002-420c-4309-bae2-4175b993f8f1
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, asks=3c8d4c9c-abca-4faa-b7c1-aae39951376c, brief=1d9df88a-40f6-4235-a083-e063fde634fe, decisions=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, primary=c6fc0041-dead-4e31-9c19-8913c1bdfa29, record=68536615-d17e-4399-ad6c-2a2cf8622b2b, satellite=dd50488e-a8cd-4778-aecc-72295ffd8671

Problem: an agent in a satellite sees decisions and cannot tell where they are written.
Standard: the tool says what the repository is; the brief says only how to work.

- A) Yes.
  Why: the first line of show says where the record is; the brief stays the same in every repository.
  Costs: one line more on every show in a satellite.
- B) No.
  Why: show prints the same in every repository.
  Costs: the brief has to carry a repository-specific line, or the agent asks.

### Does check in a satellite report an untested decision?
depends: dd50488e-a8cd-4778-aecc-72295ffd8671, 41c41584-1ea3-4f15-b7ec-76a1dc99d32f, 13ddec51-ce03-4645-99ae-2bbf4f30712d=A
unrelated: 3460d220-e523-448b-aeff-baa8051b7d06, 2381d759-f85f-484e-9031-fadbe6ea045a, 0c3d498a-c540-47f3-8384-050fc044c109, 80b89c24-7d65-43f4-bb03-55e9d85c327d, db07a54d-4d80-4035-a9ef-eabcc0c872b3, bd83aa22-94fc-49ac-b215-a82990ff8c65, ed372cc6-c161-4b90-85bd-c73a6e788c5e, a469ecb9-a4f1-4a2a-89c5-7378cc1138b4, 912f4f3b-1213-435a-af0d-7617017fcff4, 48c47c42-6720-44ca-b47d-12a61ba8ee9f, fc118154-4172-48db-bddb-75a691894edf, a8280dd1-4bfa-4eeb-9656-1cd06381590a, 7e756e04-dde0-4137-ad4f-646e52073398, 796a2544-43e5-43db-b5d3-8a4e737ec9c6, 34e81f57-9b3c-4dc4-a2ba-ca227523177d, 1c27f2d6-ab62-4c5a-91b2-1488722f3c48, f8d92841-aef7-4cd7-bd94-0e07c03e4cb6, c3127af2-4c96-48d5-a164-bac49a9a09ec, 4319a653-6901-446c-94ab-6420ef472134, 9a2ff21b-c20b-4c63-95f2-a7a3cbe236b8, 0e16fbf3-1401-4538-bd12-25303994af10, 03f47941-43c5-479f-bf64-2d030361092d, 937bc0a6-691d-4d67-91c6-cc9e956ebc96, 30063002-420c-4309-bae2-4175b993f8f1, da777c6e-f22a-41fa-ba6f-1f1544382710, cb15b115-610f-4c82-a8f7-6bae9c131dc0
id: 563ba307-4c7d-4945-9c89-46dfa61cb88d
terms: builder=c49997ee-10e7-48e0-b66b-43f8b09758a6, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, moves=d904dc0d-231f-4e93-ac74-7babe888107d, primary=c6fc0041-dead-4e31-9c19-8913c1bdfa29, satellite=dd50488e-a8cd-4778-aecc-72295ffd8671

Problem: a satellite sees its own tests only; a decision tested in another repository looks untested there.
Standard: a coverage report is made where every tree is visible, and nowhere else.

- A) No.
  Why: the build moves come from the primary's show, which sees every satellite.
  Costs: a builder in a satellite reads the primary's show to learn what to build.
- B) Yes.
  Why: every repository reports the same.
  Costs: a false report in every satellite for every decision built elsewhere.

### What does check in the primary report for a satellite missing at its path?
depends: c6fc0041-dead-4e31-9c19-8913c1bdfa29, dd50488e-a8cd-4778-aecc-72295ffd8671, ce1bfaa9-0ae8-4065-b8f5-4dc8a67fa862, 13ddec51-ce03-4645-99ae-2bbf4f30712d=A
unrelated: 0c3d498a-c540-47f3-8384-050fc044c109, db07a54d-4d80-4035-a9ef-eabcc0c872b3, c5e10ae8-258d-4f29-b9fe-13cb7ad2a550, a469ecb9-a4f1-4a2a-89c5-7378cc1138b4, 2381d759-f85f-484e-9031-fadbe6ea045a, 05141f49-d9d2-407c-b60c-ebb195a05864, bd83aa22-94fc-49ac-b215-a82990ff8c65, f53ac3b9-efde-427c-8027-19988aa8fda7, d3be71e4-ce7b-43b0-86a8-e86868521f99, a8280dd1-4bfa-4eeb-9656-1cd06381590a, 7e756e04-dde0-4137-ad4f-646e52073398, c10c3a49-e993-4c9f-a46b-4a1ba3646dcf, c3127af2-4c96-48d5-a164-bac49a9a09ec, 4319a653-6901-446c-94ab-6420ef472134, 3460d220-e523-448b-aeff-baa8051b7d06, 0e16fbf3-1401-4538-bd12-25303994af10, 03f47941-43c5-479f-bf64-2d030361092d, 563ba307-4c7d-4945-9c89-46dfa61cb88d, 937bc0a6-691d-4d67-91c6-cc9e956ebc96, 30063002-420c-4309-bae2-4175b993f8f1, da777c6e-f22a-41fa-ba6f-1f1544382710, cb15b115-610f-4c82-a8f7-6bae9c131dc0
id: fc118154-4172-48db-bddb-75a691894edf
terms: builder=c49997ee-10e7-48e0-b66b-43f8b09758a6, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, decision=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, decisions=41c41584-1ea3-4f15-b7ec-76a1dc99d32f, finding=ce1bfaa9-0ae8-4065-b8f5-4dc8a67fa862, move=d904dc0d-231f-4e93-ac74-7babe888107d, primary=c6fc0041-dead-4e31-9c19-8913c1bdfa29, satellite=dd50488e-a8cd-4778-aecc-72295ffd8671

Problem: with a satellite absent the count of ties is short, and every decision built there reads as untested.
Standard: a report over a partial input says the input is partial and makes no claim from it.

- A) That the satellite is not found, with its url; no untested decision until it is.
  Why: nothing is there to test; a build move from half a tree would send a builder to build twice.
  Costs: a satellite nobody clones hides the untested decisions of every repository.
- B) That the satellite is not found, with its url; the untested decisions as usual.
  Why: every finding is printed.
  Costs: a build move for every decision built in the absent satellite.
- C) Nothing; the count is what it is.
  Why: check reads what is there.
  Costs: a wrong build move with no word on why.

### Does check hold a satellite's brief to the primary's?
depends: dd50488e-a8cd-4778-aecc-72295ffd8671, c6fc0041-dead-4e31-9c19-8913c1bdfa29, 1d9df88a-40f6-4235-a083-e063fde634fe, a8280dd1-4bfa-4eeb-9656-1cd06381590a=A
unrelated: 6ef9eca6-fd8b-4211-aae7-00b37a2a9245, ec3f875c-a0f8-4085-8da1-fec9dcb14377, 80b89c24-7d65-43f4-bb03-55e9d85c327d, e3200081-9995-4bbe-983f-2f430dc341e7, 0c3d498a-c540-47f3-8384-050fc044c109, db07a54d-4d80-4035-a9ef-eabcc0c872b3, ce19c883-c61d-477b-a6d4-fee5134c6e9a, 19e1c023-a678-4842-a414-bb0e45c41601, 24bdcbde-d9f4-45a0-8c3d-37e53569e950, f53ac3b9-efde-427c-8027-19988aa8fda7, fc118154-4172-48db-bddb-75a691894edf, 7e756e04-dde0-4137-ad4f-646e52073398, 34e81f57-9b3c-4dc4-a2ba-ca227523177d, c10c3a49-e993-4c9f-a46b-4a1ba3646dcf, 76c6adc6-c018-4df4-b49a-ddcd0a03a48c, a26876ba-5789-4d05-965e-853d00d9edd2, 05141f49-d9d2-407c-b60c-ebb195a05864, 0e16fbf3-1401-4538-bd12-25303994af10, 563ba307-4c7d-4945-9c89-46dfa61cb88d, 13ddec51-ce03-4645-99ae-2bbf4f30712d, 937bc0a6-691d-4d67-91c6-cc9e956ebc96, 30063002-420c-4309-bae2-4175b993f8f1, da777c6e-f22a-41fa-ba6f-1f1544382710, cb15b115-610f-4c82-a8f7-6bae9c131dc0
id: 03f47941-43c5-479f-bf64-2d030361092d
terms: agent=f90dbd5b-8fbe-4899-8588-f8762795c2e2, brief=1d9df88a-40f6-4235-a083-e063fde634fe, check=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, checked=78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, install=1b98cdea-dc1b-4789-8f67-68d130d7a49d, move=d904dc0d-231f-4e93-ac74-7babe888107d, primary=c6fc0041-dead-4e31-9c19-8913c1bdfa29, satellite=dd50488e-a8cd-4778-aecc-72295ffd8671

Problem: the copy of the brief in a satellite goes stale when the primary's changes, and nothing says so.
Standard: a copied configuration is checked against its source on every run.

- A) Yes.
  Why: check fails when the copy differs from the primary's file; the move is running install in the satellite.
  Costs: a brief change in the primary stops every satellite's commit until install runs there.
- B) No.
  Why: a copy is written once.
  Costs: a satellite's agent reads a brief the primary left behind.

### Does uninstall in a satellite drop its row from the primary?
depends: dd50488e-a8cd-4778-aecc-72295ffd8671, c6fc0041-dead-4e31-9c19-8913c1bdfa29, a8280dd1-4bfa-4eeb-9656-1cd06381590a=*, 7e756e04-dde0-4137-ad4f-646e52073398=A|B
unrelated: 292359a4-0512-465b-b6a7-79838e66e5a5, c304b7da-1843-43ad-930c-b01b3310483d, aac5c313-dedd-466b-a0c3-ead5e99ce913, a8034401-420c-45d7-8f43-60c1b9f3baf9, 459c1b77-82dd-469f-a60c-784b6f3d7b81, f53ac3b9-efde-427c-8027-19988aa8fda7, 8081c82d-e5c6-485b-a302-07d586f1a01b, 385a57d5-5f76-454a-ae5d-9b0d6f5c1492, e1c305af-af94-494a-9fdb-3faffafa0e06, b0e865b4-9204-4c55-9f5b-dcb755b3a03b, fc118154-4172-48db-bddb-75a691894edf, dfd32ef6-c2ac-40ef-b8b3-9c58732516b1, 7cf81ded-be48-4777-b83e-f8a2998c1ab5, 21ae8ae0-6884-4d3c-8464-024a8a5aae2d, 6ef9eca6-fd8b-4211-aae7-00b37a2a9245, db56a285-946f-48b1-b073-bb1e90192185, ef7ec04d-9cdd-40a1-8246-42a4a0200828, 0e16fbf3-1401-4538-bd12-25303994af10, 03f47941-43c5-479f-bf64-2d030361092d, 563ba307-4c7d-4945-9c89-46dfa61cb88d, 13ddec51-ce03-4645-99ae-2bbf4f30712d, 937bc0a6-691d-4d67-91c6-cc9e956ebc96, 30063002-420c-4309-bae2-4175b993f8f1, cb15b115-610f-4c82-a8f7-6bae9c131dc0
id: da777c6e-f22a-41fa-ba6f-1f1544382710
terms: install=1b98cdea-dc1b-4789-8f67-68d130d7a49d, owner=1d7e59ea-99e3-4d4b-9928-5dabca62723d, primary=c6fc0041-dead-4e31-9c19-8913c1bdfa29, satellite=dd50488e-a8cd-4778-aecc-72295ffd8671

Problem: a satellite uninstalled stays on the primary's list, and the primary reports it missing for ever.
Standard: what one command writes on two sides, its reverse removes on both.

- A) Yes.
  Why: install wrote both sides; uninstall leaves neither.
  Costs: the primary's checkout at its path when uninstall runs.
- B) No.
  Why: uninstall touches one repository.
  Costs: the owner edits the primary's list by hand.
