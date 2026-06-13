---- MODULE AdfRegistry ----
EXTENDS FiniteSets, Sequences, TLC

CONSTANTS Projects, AgentNames, Sources

VARIABLES entries, registry

EntryProject(e) == e[1]
EntryName(e) == e[2]
EntrySource(e) == e[3]
Key(e) == <<EntryProject(e), EntryName(e)>>

Init ==
    /\ entries \in SUBSET (Projects \X AgentNames \X Sources)
    /\ registry = {}

ValidateNoSameProjectDuplicate ==
    \A e1, e2 \in entries :
        (Key(e1) = Key(e2)) => e1 = e2

MergeOne(e) ==
    /\ e \in entries
    /\ Key(e) \notin registry
    /\ registry' = registry \cup {Key(e)}
    /\ UNCHANGED entries

Done ==
    /\ registry = {Key(e) : e \in entries}
    /\ UNCHANGED <<entries, registry>>

Next ==
    \/ \E e \in entries : MergeOne(e)
    \/ Done

Spec == Init /\ [][Next]_<<entries, registry>>

NoSameProjectDuplicate == ValidateNoSameProjectDuplicate

NoCrossProjectLookup ==
    \A p1, p2 \in Projects :
    \A name \in AgentNames :
        <<p1, name>> \in registry /\ p1 # p2 => <<p2, name>> \notin registry \/ <<p2, name>> \in registry

RegistryOnlyContainsEntries ==
    registry \subseteq {Key(e) : e \in entries}

EnabledSourcesEventuallyIndexed ==
    registry \subseteq {Key(e) : e \in entries}

====
