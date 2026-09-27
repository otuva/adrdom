# Blame

How the text of an origin enters the records so that every line of it is accounted for: a blame file under adrdom/blame holds, for one origin, its line ranges and what each range went to. The map is the proof; git keeps the text. Agents write and verify the map, the owner approves it.

### What does a blame file hold?
depends: 108053a8-c2c3-4561-874c-2f573a5409f2, ecb18413-cb3e-437e-9444-3cdd406ed67d
id: 34f0202a-0be4-4049-94de-d3886f4f8ffc

### Does a blame file cover its whole origin?
depends: 108053a8-c2c3-4561-874c-2f573a5409f2, ecb18413-cb3e-437e-9444-3cdd406ed67d
id: 5c399a14-1e5c-48eb-b502-b57524c01c24

### What does a blame row name?
depends: 108053a8-c2c3-4561-874c-2f573a5409f2, 41c41584-1ea3-4f15-b7ec-76a1dc99d32f, f590059d-b681-40ec-8a1c-c8916d71f6bb, 08193e75-dd89-4857-8e9f-b19e1a9e75e6
id: 38216d39-b064-4927-ab60-951a3c89e388

### Does the owner approve a blame file?
depends: 108053a8-c2c3-4561-874c-2f573a5409f2, e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd, 1d7e59ea-99e3-4d4b-9928-5dabca62723d
id: e879fe72-ec07-48cd-9902-2b158aaae726

### Does check hold a blame file to its origin?
depends: 34f0202a-0be4-4049-94de-d3886f4f8ffc, 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, ecb18413-cb3e-437e-9444-3cdd406ed67d
id: 5fddeeb5-cf54-4550-a701-dd9e4a6a81d6

### Does an origin leave the repository while its blame file has an open row?
depends: 38216d39-b064-4927-ab60-951a3c89e388, e879fe72-ec07-48cd-9902-2b158aaae726, ecb18413-cb3e-437e-9444-3cdd406ed67d
id: 4bb5ef1b-da16-4206-a7d2-c7380c36a4cc

### Does the tool stop while a blame file waits for approval?
depends: e879fe72-ec07-48cd-9902-2b158aaae726, 108053a8-c2c3-4561-874c-2f573a5409f2, 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de
id: 9c3cc17e-33f4-4855-b761-f8f82381cdca

### Does show print a blame file?
depends: 38216d39-b064-4927-ab60-951a3c89e388, 108053a8-c2c3-4561-874c-2f573a5409f2
id: 7b05f8df-09a5-404e-a0f0-3dd8824d1d15
