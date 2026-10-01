Standalone description produced with Mistral vision (mistral-medium-latest, chat completions, image as base64 data URI), prompted to describe only what is visible with no source context. Reference read of the published PDF crop fig-2 (caption excluded); known vision quirks: hatch placement and bar-height values are approximate.

The image shows a sequence of four panels labeled (i) through (iv) from left to right, illustrating changes in the states of five servers S1 to S5.

- Panel (i):
  - S1: a hatched cell followed by cells containing 2 and 3.
  - S2: cells containing 1, 2, 3.
  - S3: cells containing 1, 2, 3.
  - S4: two empty cells.
  - S5: two empty cells.
  - Below: text reads "(i) S4, S5 lagging; 1 faulty at S1".

- An arrow points from panel (i) to panel (ii).

- Panel (ii):
  - S1: a hatched cell followed by cells containing 2 and 3.
  - S2: cells containing 1, 2, 3 (shaded).
  - S3: cells containing 1, 2, 3 (shaded).
  - S4: two empty cells.
  - S5: two empty cells.
  - Below: text reads "(ii) S1 truncates 1,2,3; S2, S3 down".

- An arrow points from panel (ii) to panel (iii).

- Panel (iii):
  - S1: cells containing x, y, z.
  - S2: cells containing 1, 2, 3 (shaded).
  - S3: cells containing 1, 2, 3 (shaded).
  - S4: cells containing x, y, z.
  - S5: cells containing x, y, z.
  - Below: text reads "(iii) 1,2,3 lost overwritten by x,y,z".

- An arrow points from panel (iii) to panel (iv).

- Panel (iv):
  - S1: cells containing x, y, z.
  - S2: cells containing x, y, z.
  - S3: cells containing x, y, z.
  - S4: cells containing x, y, z.
  - S5: cells containing x, y, z.
  - Below: text reads "(iv) S2, S3 follow S1's log".
