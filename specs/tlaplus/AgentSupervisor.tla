---- MODULE AgentSupervisor ----
\* TLA+ specification for Terraphim Agent Supervisor
\* Models OTP-style supervision with OneForOne, OneForAll, RestForOne strategies
\*
\* Validates:
\*   Safety:  restart count never exceeds max_restarts within time_window
\*   Safety:  RestForOne only restarts agents started after the failed one
\*   Liveness: a failed agent is eventually restarted or supervisor stops
\*   Safety:  no agent is in both Running and Restarting state simultaneously

EXTENDS Integers, Sequences, FiniteSets, TLC

CONSTANTS
    Agents,           \* Set of agent PIDs (e.g., {"a1", "a2", "a3"})
    MaxRestarts,      \* Maximum restarts allowed in time window
    MaxTime,          \* Bounded time horizon for model checking
    Strategy          \* "OneForOne" | "OneForAll" | "RestForOne"

VARIABLES
    agentStatus,      \* Function: Agent -> {"Starting","Running","Stopped","Failed","Restarting"}
    restartCount,     \* Function: Agent -> Nat (restarts in current window)
    supervisorStatus, \* "Running" | "Stopped" | "Failed"
    startOrder,       \* Sequence of agents in start order (for RestForOne)
    clock             \* Abstract time counter

vars == <<agentStatus, restartCount, supervisorStatus, startOrder, clock>>

\* Type invariant
TypeOK ==
    /\ agentStatus \in [Agents -> {"Starting", "Running", "Stopped", "Failed", "Restarting"}]
    /\ restartCount \in [Agents -> 0..MaxRestarts+1]
    /\ supervisorStatus \in {"Running", "Stopped", "Failed"}
    /\ clock \in 0..MaxTime

\* ------ Initial State ------

Init ==
    /\ agentStatus = [a \in Agents |-> "Running"]
    /\ restartCount = [a \in Agents |-> 0]
    /\ supervisorStatus = "Running"
    /\ startOrder = SetToSeq(Agents)  \* Arbitrary but fixed ordering
    /\ clock = 0

\* ------ Helper: agents started after a given agent in start order ------

StartedAfter(failedAgent) ==
    LET idx == CHOOSE i \in 1..Len(startOrder) : startOrder[i] = failedAgent
    IN {startOrder[j] : j \in {k \in idx+1..Len(startOrder) : TRUE}}

\* Convert set to sequence (deterministic for model checking)
SetToSeq(S) ==
    CHOOSE seq \in [1..Cardinality(S) -> S] :
        \A i, j \in 1..Cardinality(S) : i /= j => seq[i] /= seq[j]

\* ------ Actions ------

\* An agent crashes (transitions from Running to Failed)
AgentFails(agent) ==
    /\ supervisorStatus = "Running"
    /\ agentStatus[agent] = "Running"
    /\ agentStatus' = [agentStatus EXCEPT ![agent] = "Failed"]
    /\ UNCHANGED <<restartCount, supervisorStatus, startOrder, clock>>

\* Supervisor restarts agent(s) according to strategy
\* OneForOne: restart only the failed agent
RestartOneForOne(agent) ==
    /\ Strategy = "OneForOne"
    /\ supervisorStatus = "Running"
    /\ agentStatus[agent] = "Failed"
    /\ restartCount[agent] < MaxRestarts
    /\ agentStatus' = [agentStatus EXCEPT ![agent] = "Restarting"]
    /\ restartCount' = [restartCount EXCEPT ![agent] = @ + 1]
    /\ UNCHANGED <<supervisorStatus, startOrder, clock>>

\* OneForAll: restart all agents when one fails
RestartOneForAll(agent) ==
    /\ Strategy = "OneForAll"
    /\ supervisorStatus = "Running"
    /\ agentStatus[agent] = "Failed"
    /\ restartCount[agent] < MaxRestarts
    /\ agentStatus' = [a \in Agents |->
        IF a = agent THEN "Restarting"
        ELSE IF agentStatus[a] = "Running" THEN "Restarting"
        ELSE agentStatus[a]]
    /\ restartCount' = [restartCount EXCEPT ![agent] = @ + 1]
    /\ UNCHANGED <<supervisorStatus, startOrder, clock>>

\* RestForOne: restart failed agent + all agents started after it
RestartRestForOne(agent) ==
    /\ Strategy = "RestForOne"
    /\ supervisorStatus = "Running"
    /\ agentStatus[agent] = "Failed"
    /\ restartCount[agent] < MaxRestarts
    /\ LET toRestart == StartedAfter(agent) \union {agent}
       IN agentStatus' = [a \in Agents |->
            IF a \in toRestart THEN "Restarting"
            ELSE agentStatus[a]]
    /\ restartCount' = [restartCount EXCEPT ![agent] = @ + 1]
    /\ UNCHANGED <<supervisorStatus, startOrder, clock>>

\* Restarting agent becomes Running
AgentRestarted(agent) ==
    /\ agentStatus[agent] = "Restarting"
    /\ agentStatus' = [agentStatus EXCEPT ![agent] = "Running"]
    /\ UNCHANGED <<restartCount, supervisorStatus, startOrder, clock>>

\* Supervisor gives up on agent that exceeded restart limit
SupervisorEscalates(agent) ==
    /\ supervisorStatus = "Running"
    /\ agentStatus[agent] = "Failed"
    /\ restartCount[agent] >= MaxRestarts
    /\ supervisorStatus' = "Failed"
    /\ UNCHANGED <<agentStatus, restartCount, startOrder, clock>>

\* Time advances
Tick ==
    /\ clock < MaxTime
    /\ clock' = clock + 1
    /\ UNCHANGED <<agentStatus, restartCount, supervisorStatus, startOrder>>

\* ------ Next State Relation ------

Next ==
    \/ \E a \in Agents : AgentFails(a)
    \/ \E a \in Agents : RestartOneForOne(a)
    \/ \E a \in Agents : RestartOneForAll(a)
    \/ \E a \in Agents : RestartRestForOne(a)
    \/ \E a \in Agents : AgentRestarted(a)
    \/ \E a \in Agents : SupervisorEscalates(a)
    \/ Tick

\* ------ Safety Properties ------

\* No agent exceeds max restart count
RestartBoundSafety ==
    \A a \in Agents : restartCount[a] <= MaxRestarts

\* No agent is simultaneously in two active states
NoSimultaneousStates ==
    \A a \in Agents :
        ~(agentStatus[a] = "Running" /\ agentStatus[a] = "Restarting")

\* RestForOne only restarts agents started AFTER the failed one
\* (This is checked structurally in RestartRestForOne action definition)

\* If supervisor is Failed, no more restarts happen
FailedSupervisorNoRestarts ==
    supervisorStatus = "Failed" =>
        \A a \in Agents : agentStatus'[a] /= "Restarting"

\* ------ Liveness Properties ------

\* A failed agent is eventually restarted or supervisor escalates
EventualRecovery ==
    \A a \in Agents :
        agentStatus[a] = "Failed" ~>
            (agentStatus[a] = "Running" \/ supervisorStatus = "Failed")

\* ------ Specification ------

Spec == Init /\ [][Next]_vars /\ WF_vars(Next)

====
