---------------------------- MODULE counterexample ----------------------------

EXTENDS VrrCoreErasApalacheCinit

(* Constant initialization state *)
ConstInit ==
  Commands = {"cmd"}
    /\ CrashRetainsDurable = FALSE
    /\ Defect = "none"
    /\ MaxEpoch = 0
    /\ MaxLogLength = 3
    /\ MaxNonce = 1
    /\ MaxViewIndex = 0
    /\ N0 = "n0"
    /\ N1 = "n1"
    /\ N2 = "n2"
    /\ N3 = "n3"
    /\ N4 = "n4"
    /\ N5 = "n5"
    /\ Scenario = "inc3"

(* Initial state [_transition(0)] *)
State0 ==
  Commands = {"cmd"}
    /\ CrashRetainsDurable = FALSE
    /\ Defect = "none"
    /\ MaxEpoch = 0
    /\ MaxLogLength = 3
    /\ MaxNonce = 1
    /\ MaxViewIndex = 0
    /\ N0 = "n0"
    /\ N1 = "n1"
    /\ N2 = "n2"
    /\ N3 = "n3"
    /\ N4 = "n4"
    /\ N5 = "n5"
    /\ Scenario = "inc3"
    /\ castOccurred = FALSE
    /\ committed = SetAsFun({ <<"n0", 2>>, <<"n1", 1>>, <<"n2", 1>> })
    /\ currentView
      = SetAsFun({ <<"n0", [era |-> 0, idx |-> 0]>>,
        <<"n1", [era |-> 0, idx |-> 0]>>,
        <<"n2", [era |-> 0, idx |-> 0]>> })
    /\ epochs = SetAsFun({ <<"n0", 0>>, <<"n1", 0>>, <<"n2", 0>> })
    /\ everEnteredViewChange = FALSE
    /\ historicalCommitted = {}
    /\ logs
      = SetAsFun({ <<
          "n0", <<
            [era |-> 0, kind |-> "genesis", value |-> "genesis-value"], [era |->
                0,
              kind |-> "command",
              value |-> "cmd"]
          >>
        >>,
        <<"n1", <<[era |-> 0, kind |-> "genesis", value |-> "genesis-value"]>>>>,
        <<"n2", <<[era |-> 0, kind |-> "genesis", value |-> "genesis-value"]>>>>
      })
    /\ messages
      = {[accepted |-> 0,
        committed |-> 2,
        entry |-> [era |-> 0, kind |-> "command", value |-> "increment-n0"],
        from |-> "n0",
        history |-> <<>>,
        nonce |-> 0,
        priorView |-> [era |-> 0, idx |-> 0],
        retained |-> [era |-> 0, idx |-> 0],
        routeEra |-> 0,
        slot |-> 2,
        to |-> "n1",
        type |-> "prepare",
        view |-> [era |-> 0, idx |-> 0]]}
    /\ overlapStreamed = FALSE
    /\ retainedView
      = SetAsFun({ <<"n0", [era |-> 0, idx |-> 0]>>,
        <<"n1", [era |-> 0, idx |-> 0]>>,
        <<"n2", [era |-> 0, idx |-> 0]>> })
    /\ status
      = SetAsFun({ <<"n0", "normal">>, <<"n1", "normal">>, <<"n2", "normal">> })

(* State1 [_transition(2)] *)
State1 ==
  Commands = {"cmd"}
    /\ CrashRetainsDurable = FALSE
    /\ Defect = "none"
    /\ MaxEpoch = 0
    /\ MaxLogLength = 3
    /\ MaxNonce = 1
    /\ MaxViewIndex = 0
    /\ N0 = "n0"
    /\ N1 = "n1"
    /\ N2 = "n2"
    /\ N3 = "n3"
    /\ N4 = "n4"
    /\ N5 = "n5"
    /\ Scenario = "inc3"
    /\ castOccurred = FALSE
    /\ committed = SetAsFun({ <<"n0", 2>>, <<"n1", 2>>, <<"n2", 1>> })
    /\ currentView
      = SetAsFun({ <<"n0", [era |-> 0, idx |-> 0]>>,
        <<"n1", [era |-> 0, idx |-> 0]>>,
        <<"n2", [era |-> 0, idx |-> 0]>> })
    /\ epochs = SetAsFun({ <<"n0", 0>>, <<"n1", 0>>, <<"n2", 0>> })
    /\ everEnteredViewChange = FALSE
    /\ historicalCommitted
      = {[entry |-> [era |-> 0, kind |-> "command", value |-> "increment-n0"],
        slot |-> 2]}
    /\ logs
      = SetAsFun({ <<
          "n0", <<
            [era |-> 0, kind |-> "genesis", value |-> "genesis-value"], [era |->
                0,
              kind |-> "command",
              value |-> "cmd"]
          >>
        >>,
        <<
          "n1", <<
            [era |-> 0, kind |-> "genesis", value |-> "genesis-value"], [era |->
                0,
              kind |-> "command",
              value |-> "increment-n0"]
          >>
        >>,
        <<"n2", <<[era |-> 0, kind |-> "genesis", value |-> "genesis-value"]>>>>
      })
    /\ messages
      = { [accepted |-> 0,
          committed |-> 0,
          entry |-> [era |-> 0, kind |-> "genesis", value |-> "genesis-value"],
          from |-> "n1",
          history |-> <<>>,
          nonce |-> 0,
          priorView |-> [era |-> 0, idx |-> 0],
          retained |-> [era |-> 0, idx |-> 0],
          routeEra |-> 0,
          slot |-> 2,
          to |-> "n0",
          type |-> "prepare-ok",
          view |-> [era |-> 0, idx |-> 0]],
        [accepted |-> 0,
          committed |-> 2,
          entry |-> [era |-> 0, kind |-> "command", value |-> "increment-n0"],
          from |-> "n0",
          history |-> <<>>,
          nonce |-> 0,
          priorView |-> [era |-> 0, idx |-> 0],
          retained |-> [era |-> 0, idx |-> 0],
          routeEra |-> 0,
          slot |-> 2,
          to |-> "n1",
          type |-> "prepare",
          view |-> [era |-> 0, idx |-> 0]] }
    /\ overlapStreamed = FALSE
    /\ retainedView
      = SetAsFun({ <<"n0", [era |-> 0, idx |-> 0]>>,
        <<"n1", [era |-> 0, idx |-> 0]>>,
        <<"n2", [era |-> 0, idx |-> 0]>> })
    /\ status
      = SetAsFun({ <<"n0", "normal">>, <<"n1", "normal">>, <<"n2", "normal">> })

(* The following formula holds true in the last state and violates the invariant *)
InvariantViolation ==
  Skolem((\E r_46 \in { "n0", "n1", "n2" }:
    Skolem((\E s_8 \in { "n0", "n1", "n2" }:
      Skolem((\E j_35 \in 1 .. 3:
        j_35
            <= (IF committed[r_46] <= committed[s_8]
            THEN committed[r_46]
            ELSE committed[s_8])
          /\ ~(logs[r_46][j_35] = logs[s_8][j_35])))))))

================================================================================
(* Created by Apalache on Tue Sep 08 23:14:00 UTC 2026 *)
(* https://github.com/apalache-mc/apalache *)
