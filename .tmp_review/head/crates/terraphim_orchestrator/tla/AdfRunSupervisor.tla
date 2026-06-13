---- MODULE AdfRunSupervisor ----
EXTENDS Naturals, TLC

CONSTANTS Runs, MaxRetries

VARIABLES active, retrying, terminal, escalated, statusPosted

Init ==
    /\ active = {}
    /\ retrying = [r \in Runs |-> 0]
    /\ terminal = {}
    /\ escalated = FALSE
    /\ statusPosted = {}

Spawn(r) ==
    /\ r \in Runs
    /\ r \notin active
    /\ r \notin terminal
    /\ ~escalated
    /\ active' = active \cup {r}
    /\ UNCHANGED <<retrying, terminal, escalated, statusPosted>>

FailForRetry(r) ==
    /\ r \in active
    /\ retrying[r] < MaxRetries
    /\ active' = active \ {r}
    /\ retrying' = [retrying EXCEPT ![r] = @ + 1]
    /\ UNCHANGED <<terminal, escalated, statusPosted>>

GiveUp(r) ==
    /\ r \in active
    /\ retrying[r] >= MaxRetries
    /\ active' = active \ {r}
    /\ terminal' = terminal \cup {r}
    /\ statusPosted' = statusPosted \cup {r}
    /\ UNCHANGED <<retrying, escalated>>

Complete(r) ==
    /\ r \in active
    /\ active' = active \ {r}
    /\ terminal' = terminal \cup {r}
    /\ statusPosted' = statusPosted \cup {r}
    /\ UNCHANGED <<retrying, escalated>>

Escalate ==
    /\ escalated = FALSE
    /\ \E r \in Runs : retrying[r] >= MaxRetries
    /\ escalated' = TRUE
    /\ active' = {}
    /\ UNCHANGED <<retrying, terminal, statusPosted>>

Next ==
    \/ \E r \in Runs : Spawn(r)
    \/ \E r \in Runs : FailForRetry(r)
    \/ \E r \in Runs : GiveUp(r)
    \/ \E r \in Runs : Complete(r)
    \/ Escalate

Spec == Init /\ [][Next]_<<active, retrying, terminal, escalated, statusPosted>>

RetryBound == \A r \in Runs : retrying[r] <= MaxRetries

NoRestartAfterEscalation == escalated => active = {}

TerminalStatusAtMostOnce == statusPosted \subseteq terminal

ActiveReleasedOnTerminal == active \cap terminal = {}

====
