-------------------------- MODULE VrrCoreErasApalacheCinit --------------------------
(***************************************************************************
 * Thin wrapper for Apalache CLI-driven inductive-invariant checks: supplies
 * the normative three-node scenario constants through --cinit=ConstInit so
 * that --init/--next can be given on the command line (a TLC config file
 * with a SPECIFICATION clause overrides --init, and an INIT clause breaks
 * the transition extractor on this model).
 ***************************************************************************)
EXTENDS VrrCoreErasApalache

ConstInit ==
    /\ N0 = "n0"
    /\ N1 = "n1"
    /\ N2 = "n2"
    /\ N3 = "n3"
    /\ N4 = "n4"
    /\ N5 = "n5"
    /\ Scenario = "inc3"
    /\ Commands = {"cmd"}
    /\ MaxViewIndex = 0
    /\ MaxLogLength = 3
    /\ MaxEpoch = 0
    /\ MaxNonce = 1
    /\ CrashRetainsDurable = FALSE
    /\ Defect = "none"

=============================================================================
