# Core

What the tool holds fixed in every repository: what an answered question may no longer do, who
may run the owner's commands, what a question stands on, and the order questions reach the owner.

### Does a decision change after it is answered?
depends: 41c41584-1ea3-4f15-b7ec-76a1dc99d32f, b36113a1-1384-4e4c-a798-8399f4181366, 2bae531b-c4df-48a0-89a0-3d52726741ea
id: dad5273c-afe2-4ab1-a451-e2a5d4d9a26d

### Does the tool guard against an agent that works around it on purpose?
depends: 1d7e59ea-99e3-4d4b-9928-5dabca62723d, f90dbd5b-8fbe-4899-8588-f8762795c2e2
id: 98d5d389-8315-41da-9970-666e31f3a94d

### Does ask stop a question outside the plan?
depends: 3c8d4c9c-abca-4faa-b7c1-aae39951376c, 94677ed8-eaf6-46e2-bc71-7392fa0805bf, e9a1ecc6-f300-4b04-ae85-f4b88c1ba9bd
id: da59b448-1afb-4469-b40e-69eda8f253c2

### Does check fail a current decision with no test?
depends: 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, e2533580-1486-4dc9-8697-e6c53605fe72, 41c41584-1ea3-4f15-b7ec-76a1dc99d32f
id: 33a45b9e-8150-488e-be70-7ab2ca9adaa6

### Does check fail an answered question whose text changed?
depends: dad5273c-afe2-4ab1-a451-e2a5d4d9a26d, 78f2f2d0-cbef-40ef-ae75-cc2ff3f953de, e18b82d8-5271-4385-bbab-2b89e5da278f
id: cd83e277-ebf0-4458-a77a-c526ab4cfeca

### What does an owner command look for before it runs?
depends: 98d5d389-8315-41da-9970-666e31f3a94d, 1d7e59ea-99e3-4d4b-9928-5dabca62723d, f90dbd5b-8fbe-4899-8588-f8762795c2e2
id: aa275fac-def5-42a2-8a61-660142acad8e

### What does a depends line name?
depends: da59b448-1afb-4469-b40e-69eda8f253c2, aef1b384-2b33-47a2-92a8-ec7f8fb258ef
id: ad9761e2-6d64-4b45-b0ba-5c5d8d2b38bf

### What ties a test to a decision?
depends: 33a45b9e-8150-488e-be70-7ab2ca9adaa6, 41c41584-1ea3-4f15-b7ec-76a1dc99d32f
id: 0b962bc8-05a8-48da-9fe8-bf77c7aa6e4a

### Does the tool derive tiers from the depends lines?
depends: ad9761e2-6d64-4b45-b0ba-5c5d8d2b38bf, f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c
id: aec860fd-22ad-44a7-99f7-222ea19537e7

### Does a reference in a topic file carry the identifier of what it names?
depends: ad9761e2-6d64-4b45-b0ba-5c5d8d2b38bf, e877e4a9-2caf-4fe6-9f82-5476a2abb00b, 08193e75-dd89-4857-8e9f-b19e1a9e75e6
id: 7cad9922-9bd1-4c1a-aedc-df1f77c83e1f

### Does the tool cap the questions in a tier?
depends: aec860fd-22ad-44a7-99f7-222ea19537e7, f4819229-0bf1-4ca0-a2a7-bd2f318fbe8c
id: 15d82e53-e344-4347-b639-3bf6eb9ef1b4

### Does a depends entry carry the answers it holds under?
depends: aec860fd-22ad-44a7-99f7-222ea19537e7, 9d634996-475e-4ddc-8113-05c404ad9d5c
id: fa90692a-150e-4a89-91e2-1d575cb59d0f
