---- MODULE DelayedFence ----
EXTENDS VrrCoreEras
VARIABLE pc
TraceInit == Init /\ pc = 0
Scheduled == CASE pc = 0 -> (EnterViewChange(N1, View(0,1)))
  [] pc = 1 -> (Crash(N1))
  [] pc = 2 -> (BeginRecovery(N1))
  [] pc = 3 -> (\E m \in messages : /\ m.type = RecoveryMsg /\ m.from = N1
          /\ m.to = N0 /\ m.nonce = 1 /\ RespondToRecovery(N0, m))
  [] pc = 4 -> (\E m \in messages : /\ m.type = RecoveryMsg /\ m.from = N1
          /\ m.to = N2 /\ m.nonce = 1 /\ RespondToRecovery(N2, m))
  [] pc = 5 -> (\E m \in messages : /\ m.type = StartViewChangeMsg
          /\ m.from = N1 /\ m.to = N2 /\ TRUE
          /\ FollowHigherViewChange(N2, m))
  [] pc = 6 -> (SendDoViewChange(N2))
  [] pc = 7 -> (\E m \in messages : /\ m.type = RecoveryResponseMsg
          /\ m.from = N0 /\ m.to = N1 /\ m.nonce = 1
          /\ RecordRecoveryResponse(N1, m))
  [] pc = 8 -> (\E m \in messages : /\ m.type = RecoveryResponseMsg
          /\ m.from = N2 /\ m.to = N1 /\ m.nonce = 1
          /\ RecordRecoveryResponse(N1, m))
  [] pc = 9 -> (\E m \in messages : /\ m.type = RecoveryResponseMsg
          /\ m.from = N0 /\ m.to = N1 /\ m.nonce = 1
          /\ CompleteRecovery(N1, m))
  [] pc = 10 -> (Crash(N2))
  [] pc = 11 -> (BeginRecovery(N2))
  [] pc = 12 -> (\E m \in messages : /\ m.type = RecoveryMsg /\ m.from = N2
          /\ m.to = N0 /\ m.nonce = 1 /\ RespondToRecovery(N0, m))
  [] pc = 13 -> (\E m \in messages : /\ m.type = RecoveryMsg /\ m.from = N2
          /\ m.to = N1 /\ m.nonce = 1 /\ RespondToRecovery(N1, m))
  [] pc = 14 -> (\E m \in messages : /\ m.type = RecoveryResponseMsg
          /\ m.from = N0 /\ m.to = N2 /\ m.nonce = 1
          /\ RecordRecoveryResponse(N2, m))
  [] pc = 15 -> (\E m \in messages : /\ m.type = RecoveryResponseMsg
          /\ m.from = N1 /\ m.to = N2 /\ m.nonce = 1
          /\ RecordRecoveryResponse(N2, m))
  [] pc = 16 -> (\E m \in messages : /\ m.type = RecoveryResponseMsg
          /\ m.from = N0 /\ m.to = N2 /\ m.nonce = 1
          /\ CompleteRecovery(N2, m))
  [] pc = 17 -> (ProposeCommand(N0, "x"))
  [] pc = 18 -> (\E m \in messages : /\ m.type = PrepareMsg
          /\ m.from = N0 /\ m.to = N2 /\ TRUE
          /\ ReceivePrepare(N2, m))
  [] pc = 19 -> (CommitNext(N0))
  [] pc = 20 -> (\E m \in messages : /\ m.type = CommitMsg
          /\ m.from = N0 /\ m.to = N2 /\ TRUE
          /\ ReceiveCommit(N2, m))
  [] pc = 21 -> (EnterViewChange(N1, View(0,1)))
  [] pc = 22 -> (SendDoViewChange(N1))
  [] pc = 23 -> (\E m \in messages : /\ m.type = DoViewChangeMsg
          /\ m.from = N2 /\ m.to = N1 /\ TRUE
          /\ InstallView(N1, m))
  [] pc = 24 -> (Crash(N2))
  [] pc = 25 -> (BeginRecovery(N2))
  [] pc = 26 -> (\E m \in messages : /\ m.type = RecoveryMsg /\ m.from = N2
          /\ m.to = N0 /\ m.nonce = 2 /\ RespondToRecovery(N0, m))
  [] pc = 27 -> (\E m \in messages : /\ m.type = RecoveryMsg /\ m.from = N2
          /\ m.to = N1 /\ m.nonce = 2 /\ RespondToRecovery(N1, m))
  [] pc = 28 -> (\E m \in messages : /\ m.type = RecoveryResponseMsg
          /\ m.from = N0 /\ m.to = N2 /\ m.nonce = 2
          /\ RecordRecoveryResponse(N2, m))
  [] pc = 29 -> (\E m \in messages : /\ m.type = RecoveryResponseMsg
          /\ m.from = N1 /\ m.to = N2 /\ m.nonce = 2
          /\ RecordRecoveryResponse(N2, m))
  [] pc = 30 -> (\E m \in messages : /\ m.type = RecoveryResponseMsg
          /\ m.from = N1 /\ m.to = N2 /\ m.nonce = 2
          /\ CompleteRecovery(N2, m))
  [] pc = 31 -> (ProposeCommand(N1, "y"))
  [] pc = 32 -> (\E m \in messages : /\ m.type = PrepareMsg
          /\ m.from = N1 /\ m.to = N2 /\ TRUE
          /\ ReceivePrepare(N2, m))
  [] pc = 33 -> (CommitNext(N1))
  [] OTHER -> FALSE
TraceNext == /\ Scheduled
             /\ pc' = pc + 1
             /\ historicalCommitted' = historicalCommitted \cup
                  UNION {{[slot |-> i, entry |-> logs'[r][i]] :
                    i \in (committed[r] + 1)..committed'[r]} : r \in Nodes}
TraceSpec == TraceInit /\ [][TraceNext]_<<vars,pc>>
OneRecovering == Cardinality({r \in Nodes : status[r] = Recovering}) <= 1
====
