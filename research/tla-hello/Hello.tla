---- MODULE Hello ----
EXTENDS Naturals

VARIABLE bit

Init == bit = 0
Next == bit' = 1 - bit
Inv == bit \in {0, 1}

====
