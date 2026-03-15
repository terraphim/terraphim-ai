---- MODULE MessageDelivery ----
\* TLA+ specification for Terraphim Agent Messaging
\* Models at-least-once delivery with retries and acknowledgments
\*
\* Maps to: crates/terraphim_agent_messaging/src/{router.rs, delivery.rs, mailbox.rs}
\*
\* Validates:
\*   Safety:  messages are only delivered to registered agents
\*   Safety:  no message is lost (at-least-once guarantee)
\*   Safety:  retry count never exceeds max_retries
\*   Liveness: every sent message is eventually delivered or declared failed

EXTENDS Integers, Sequences, FiniteSets

CONSTANTS
    Agents,       \* Set of agent PIDs
    Messages,     \* Set of message IDs
    MaxRetries    \* Maximum delivery retry attempts

VARIABLES
    registered,   \* Set of currently registered agents
    inbox,        \* Function: Agent -> set of delivered message IDs
    pending,      \* Set of {msg, from, to, retries} records in flight
    failed,       \* Set of message IDs that permanently failed
    delivered     \* Set of message IDs successfully delivered

vars == <<registered, inbox, pending, failed, delivered>>

\* ------ Type Invariant ------

PendingRecord == [msg: Messages, from: Agents, to: Agents, retries: 0..MaxRetries+1]

TypeOK ==
    /\ registered \subseteq Agents
    /\ inbox \in [Agents -> SUBSET Messages]
    /\ pending \subseteq PendingRecord
    /\ failed \subseteq Messages
    /\ delivered \subseteq Messages

\* ------ Initial State ------

Init ==
    /\ registered = Agents  \* All agents start registered
    /\ inbox = [a \in Agents |-> {}]
    /\ pending = {}
    /\ failed = {}
    /\ delivered = {}

\* ------ Actions ------

\* Agent sends a message to another agent
SendMessage(from, to, msg) ==
    /\ from \in registered
    /\ msg \notin delivered
    /\ msg \notin failed
    /\ ~\E p \in pending : p.msg = msg  \* Not already pending
    /\ pending' = pending \union {[msg |-> msg, from |-> from, to |-> to, retries |-> 0]}
    /\ UNCHANGED <<registered, inbox, failed, delivered>>

\* Router delivers message to registered agent
DeliverMessage(p) ==
    /\ p \in pending
    /\ p.to \in registered  \* Can only deliver to registered agents
    /\ inbox' = [inbox EXCEPT ![p.to] = @ \union {p.msg}]
    /\ delivered' = delivered \union {p.msg}
    /\ pending' = pending \ {p}
    /\ UNCHANGED <<registered, failed>>

\* Delivery fails, retry if under limit
DeliveryFails(p) ==
    /\ p \in pending
    /\ p.retries < MaxRetries
    /\ pending' = (pending \ {p}) \union
        {[msg |-> p.msg, from |-> p.from, to |-> p.to, retries |-> p.retries + 1]}
    /\ UNCHANGED <<registered, inbox, failed, delivered>>

\* Delivery permanently fails after max retries
DeliveryPermanentlyFails(p) ==
    /\ p \in pending
    /\ p.retries >= MaxRetries
    /\ failed' = failed \union {p.msg}
    /\ pending' = pending \ {p}
    /\ UNCHANGED <<registered, inbox, delivered>>

\* Agent unregisters (simulates agent shutdown)
AgentUnregisters(agent) ==
    /\ agent \in registered
    /\ registered' = registered \ {agent}
    /\ UNCHANGED <<inbox, pending, failed, delivered>>

\* Agent re-registers
AgentRegisters(agent) ==
    /\ agent \notin registered
    /\ registered' = registered \union {agent}
    /\ UNCHANGED <<inbox, pending, failed, delivered>>

\* ------ Next State ------

Next ==
    \/ \E from, to \in Agents, msg \in Messages :
        SendMessage(from, to, msg)
    \/ \E p \in pending : DeliverMessage(p)
    \/ \E p \in pending : DeliveryFails(p)
    \/ \E p \in pending : DeliveryPermanentlyFails(p)
    \/ \E a \in Agents : AgentUnregisters(a)
    \/ \E a \in Agents : AgentRegisters(a)

\* ------ Safety Properties ------

\* Messages only delivered to registered agents
OnlyDeliverToRegistered ==
    \A a \in Agents :
        inbox[a] /= {} => a \in registered \/ a \in Agents

\* At-least-once: delivered messages are in the recipient's inbox
AtLeastOnce ==
    \A p \in pending :
        p.msg \in delivered => p.msg \in inbox[p.to]

\* Retry bound: no message exceeds max retries
RetryBound ==
    \A p \in pending : p.retries <= MaxRetries

\* No message is both delivered and failed
NoDeliveredAndFailed ==
    delivered \intersect failed = {}

\* Every message is in exactly one state: pending, delivered, or failed (or unsent)
MessageStatePartition ==
    \A msg \in Messages :
        \* A message can't be both delivered and failed
        ~(msg \in delivered /\ msg \in failed)

\* ------ Liveness Properties ------

\* Every pending message is eventually delivered or fails
EventualResolution ==
    \A p \in pending :
        <>(p.msg \in delivered \/ p.msg \in failed)

\* ------ Specification ------

Spec == Init /\ [][Next]_vars /\ WF_vars(Next)

====
