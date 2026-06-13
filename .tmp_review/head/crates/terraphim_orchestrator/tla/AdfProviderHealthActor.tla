---- MODULE AdfProviderHealthActor ----
EXTENDS Naturals, TLC

CONSTANTS Providers

VARIABLES probing, cached, lifecycleEvents

Init ==
    /\ probing = {}
    /\ cached = {}
    /\ lifecycleEvents = 0

StartProbe(p) ==
    /\ p \in Providers
    /\ p \notin probing
    /\ probing' = probing \cup {p}
    /\ UNCHANGED <<cached, lifecycleEvents>>

FinishProbe(p) ==
    /\ p \in probing
    /\ probing' = probing \ {p}
    /\ cached' = cached \cup {p}
    /\ UNCHANGED lifecycleEvents

TimeoutProbe(p) ==
    /\ p \in probing
    /\ probing' = probing \ {p}
    /\ UNCHANGED <<cached, lifecycleEvents>>

ProcessLifecycleEvent ==
    /\ lifecycleEvents' = lifecycleEvents + 1
    /\ UNCHANGED <<probing, cached>>

Next ==
    \/ \E p \in Providers : StartProbe(p)
    \/ \E p \in Providers : FinishProbe(p)
    \/ \E p \in Providers : TimeoutProbe(p)
    \/ ProcessLifecycleEvent

Spec == Init /\ [][Next]_<<probing, cached, lifecycleEvents>>

ProbeDelayDoesNotBlockLifecycle == lifecycleEvents \in Nat

CachedOnlyKnownProviders == cached \subseteq Providers

ProbingOnlyKnownProviders == probing \subseteq Providers

====
