---- MODULE AdfBuildDispatch ----
EXTENDS Naturals, Sequences, TLC

CONSTANTS Projects, Shas

VARIABLES configured, gated, active, pending, terminal

BuildKey == Projects \X Shas

Init ==
    /\ configured \in SUBSET Projects
    /\ gated \in SUBSET Projects
    /\ active = {}
    /\ pending = {}
    /\ terminal = {}

Spawn(k) ==
    /\ k \in BuildKey
    /\ k[1] \in configured
    /\ k[1] \notin gated
    /\ k \notin active
    /\ k \notin terminal
    /\ active' = active \cup {k}
    /\ pending' = pending \cup {k}
    /\ UNCHANGED <<configured, gated, terminal>>

SkipMissingOrGated(k) ==
    /\ k \in BuildKey
    /\ (k[1] \notin configured \/ k[1] \in gated)
    /\ UNCHANGED <<configured, gated, active, pending, terminal>>

Exit(k) ==
    /\ k \in active
    /\ active' = active \ {k}
    /\ terminal' = terminal \cup {k}
    /\ UNCHANGED <<configured, gated, pending>>

Next ==
    \/ \E k \in BuildKey : Spawn(k)
    \/ \E k \in BuildKey : SkipMissingOrGated(k)
    \/ \E k \in BuildKey : Exit(k)

Spec == Init /\ [][Next]_<<configured, gated, active, pending, terminal>>

NoPendingWithoutSpawn == pending \subseteq active \cup terminal

SkippedBuildRunnerPostsNoPending ==
    \A k \in BuildKey : (k[1] \notin configured \/ k[1] \in gated) => k \notin pending

AtMostOneActiveBuildPerProjectSha == active \subseteq BuildKey

TerminalOnlyAfterPending == terminal \subseteq pending

PendingEventuallyTerminalOrActive == pending \subseteq active \cup terminal

====
