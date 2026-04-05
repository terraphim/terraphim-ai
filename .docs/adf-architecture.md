# AI Dark Factory (ADF) Architecture

## ASCII Overview

```
+=========================================================================+
|                     AI DARK FACTORY ORCHESTRATOR                        |
|                        (bigbox systemd service)                         |
+=========================================================================+
|                                                                         |
|  +------------------+   +------------------+   +---------------------+  |
|  |  MENTION POLLING  |   |     WEBHOOK      |   |  CRON SCHEDULER    |  |
|  |  (every 2 ticks)  |   | 172.18.0.1:9091  |   |  (per-agent cron)  |  |
|  |                   |   |  HMAC-SHA256 sig  |   |                    |  |
|  | Scans comments    |   |  Real-time push   |   | Fires agents on   |  |
|  | for @adf:agent    |   |  from Gitea       |   | schedule           |  |
|  +--------+----------+   +--------+----------+   +---------+----------+  |
|           |                       |                         |            |
|           +------------+----------+----------+--------------+            |
|                        |                     |                           |
|                        v                     v                           |
|              +---------+--------+   +--------+---------+                 |
|              |  AGENT DISPATCH  |   | COMPOUND REVIEW  |                 |
|              |                  |   |  (hourly swarm)  |                 |
|              | 1. Clone def     |   |  6-agent review  |                 |
|              | 2. Inject ctx    |   |  in git worktree |                 |
|              | 3. Spawn process |   |  files findings  |                 |
|              | 4. Assign issue  |   |  as Gitea issues |                 |
|              +---------+--------+   +------------------+                 |
|                        |                                                 |
|  +---------------------+---------------------------------------------+  |
|  |                    AGENT OUTPUT PIPELINE                           |  |
|  |                                                                    |  |
|  |  OutputPoster                                                      |  |
|  |  +-- default_tracker (root token)                                  |  |
|  |  +-- agent_trackers (13 per-agent tokens from agent_tokens.json)   |  |
|  |      Each agent posts comments under its own Gitea user            |  |
|  +--------------------------------------------------------------------+  |
|                                                                         |
+=========================================================================+


================================ AGENT LAYERS ================================

SAFETY LAYER (always-on watchdogs)
+---------------------+   +---------------------+
| security-sentinel   |   | drift-detector      |
| Every 6h + mention  |   | Every 12h           |
| cargo audit, CVE    |   | Config drift scan   |
| Posts to issue #108  |   |                     |
+---------------------+   +---------------------+

CORE LAYER (essential pipeline agents)
+---------------------+   +---------------------+   +---------------------+
| test-guardian       |   | compliance-watchdog |   | spec-validator      |
| Every 8h + mention  |   | Daily 03:00 + ment. |   | Daily 03:00 + ment. |
| cargo test + clippy |   | License audit       |   | Spec vs impl check  |
| Verdict: PASS/FAIL  |   | Verdict: PASS/FAIL  |   | Verdict: PASS/FAIL  |
+---------------------+   +---------------------+   +---------------------+
+---------------------+   +---------------------+   +---------------------+
| implementation-swarm|   | upstream-sync       |   | meta-coordinator    |
| Midnight + mention  |   | Every 6h            |   | Every 12h           |
| Branch, code, PR    |   | Check upstream deps |   | ADF health monitor  |
| TDD workflow        |   | Flag risky commits  |   | Creates P0/P1 issues|
+---------------------+   +---------------------+   +---------------------+
+---------------------+   +---------------------+
| documentation-gen   |   | product-development |
| Daily 04:00         |   | Every 6h            |
| Doc comments, CHLOG |   | Code review         |
| Posts to issue #114  |   | Posts to issue #112  |
+---------------------+   +---------------------+

GROWTH LAYER (coordination and quality)
+---------------------+   +---------------------+   +---------------------+
| quality-coordinator |   | merge-coordinator   |   | browser-qa          |
| Mention-only        |   | Mention-only        |   | Mention-only        |
| Compound code review|   | 5-verdict merge gate|   | Playwright visual   |
| Triggers 4 reviewers|   | Creates remediation |   | testing             |
|                     |   | issues for FAILs    |   |                     |
+---------------------+   +---------------------+   +---------------------+

FLOW DAG (multi-step pipeline)
+---------------------------------------------------------------------+
| security-audit-flow  (every 6h)                                     |
|                                                                      |
|  [cargo-audit] ---> [check-findings] ---> [analyse-findings] ---->  |
|   action: run        gate: exit!=0?        agent: LLM analysis      |
|   cargo audit        yes: continue         categorise P0-P3         |
|                      no: abort             write report              |
|                                                                      |
|  ----> [post-to-gitea]                                              |
|          action: gtr comment to #108                                 |
+---------------------------------------------------------------------+


========================== REVIEWER CHAIN FLOW ===========================

  Issue created (finding or manual)
       |
       v
  @adf:implementation-swarm  ------>  Branch + implement + PR
       |
       v
  @adf:quality-coordinator  ------>  Compound code review
       |
       | (if GO)
       v
  Triggers 4 reviewers on same issue:
  +---> @adf:test-guardian       ----> cargo test, verdict PASS/FAIL
  +---> @adf:security-sentinel   ----> cargo audit, verdict PASS/FAIL
  +---> @adf:spec-validator      ----> spec check, verdict PASS/FAIL
  +---> @adf:compliance-watchdog ----> license audit, verdict PASS/FAIL
       |
       | Each posts verdict + @adf:merge-coordinator
       v
  merge-coordinator checks all 5 verdicts:
       |
       +--- All PASS ---> Merge PR, close issue
       |
       +--- Any FAIL ---> Create [Remediation] issues
       |                  per FAIL verdict
       |                  Trigger @adf:implementation-swarm
       |                  on each remediation issue
       |
       +--- Any MISSING -> Post "Cannot merge", wait


========================= INFRASTRUCTURE ==================================

  Gitea (git.terraphim.cloud)          ADF Orchestrator (bigbox)
  +----------------------------+       +---------------------------+
  | Docker: gitea:1.26.0       |       | systemd: adf-orchestrator |
  | Postgres, SeaweedFS S3     |       | Binary: /usr/local/bin/adf|
  | Robot API: /api/v1/robot/* |       | Config: orchestrator.toml |
  | PageRank issue scoring     |       | Tokens: agent_tokens.json |
  | Webhook --> ADF:9091       |       | WorkDir: /opt/ai-dark-fac |
  | 29 users (13 agent users)  |       | Memory: 16G max           |
  +----------------------------+       | CPU: 400% (4 cores)       |
           |                            +---------------------------+
           | HTTPS API                             |
           +<--------------------------------------+
             Comments, issues, PRs, assignments
```

## Mermaid Diagram

```mermaid
%%{init: {'theme': 'base', 'themeVariables': {'primaryColor': '#2d333b', 'primaryTextColor': '#adbac7', 'lineColor': '#768390'}}}%%
flowchart TB
    subgraph Gitea["Gitea (git.terraphim.cloud)"]
        issues["Issues + Comments"]
        webhook_out["Webhook Events"]
        robot_api["Robot API<br/>(PageRank)"]
    end

    subgraph ADF["ADF Orchestrator (systemd on bigbox)"]
        direction TB

        subgraph triggers["Event Sources"]
            mention["Mention Polling<br/>@adf:agent-name<br/>(every 2 ticks)"]
            webhook_in["Webhook Receiver<br/>172.18.0.1:9091<br/>(HMAC-SHA256)"]
            cron["Cron Scheduler<br/>(per-agent schedules)"]
            compound["Compound Review<br/>(hourly, 6-agent swarm)"]
        end

        dispatch["Agent Dispatch<br/>1. Clone definition<br/>2. Inject mention context<br/>3. Spawn CLI process<br/>4. Assign Gitea issue"]

        subgraph output["Output Pipeline"]
            poster["OutputPoster<br/>13 per-agent tokens<br/>+ 1 default (root)"]
        end

        subgraph safety["Safety Layer"]
            sentinel["security-sentinel<br/>Every 6h + mention<br/>cargo audit, CVE scan"]
            drift["drift-detector<br/>Every 12h<br/>Config drift"]
        end

        subgraph core["Core Layer"]
            test_g["test-guardian<br/>Every 8h + mention<br/>cargo test + clippy"]
            compliance["compliance-watchdog<br/>Daily 03:00 + mention<br/>License audit"]
            spec["spec-validator<br/>Daily 03:00 + mention<br/>Spec vs implementation"]
            impl["implementation-swarm<br/>Midnight + mention<br/>Branch, code, PR, TDD"]
            upstream["upstream-synchronizer<br/>Every 6h"]
            meta["meta-coordinator<br/>Every 12h<br/>ADF health monitor"]
            docs["documentation-generator<br/>Daily 04:00"]
            product["product-development<br/>Every 6h<br/>Code review"]
        end

        subgraph growth["Growth Layer"]
            qc["quality-coordinator<br/>Mention-only<br/>Compound code review"]
            mc["merge-coordinator<br/>Mention-only<br/>5-verdict merge gate"]
            bqa["browser-qa<br/>Mention-only<br/>Playwright visual QA"]
        end

        subgraph flow["Flow DAG: security-audit-flow (every 6h)"]
            cargo_audit["cargo-audit<br/>(action)"]
            check["check-findings<br/>(gate)"]
            analyse["analyse-findings<br/>(LLM agent)"]
            post_gitea["post-to-gitea<br/>(action)"]
            cargo_audit --> check --> analyse --> post_gitea
        end
    end

    webhook_out -->|"push events"| webhook_in
    issues -->|"poll comments"| mention
    cron -->|"schedule fires"| dispatch
    mention -->|"@adf: detected"| dispatch
    webhook_in -->|"@adf: detected"| dispatch
    compound -->|"hourly"| dispatch

    dispatch --> safety
    dispatch --> core
    dispatch --> growth

    safety -->|"post as agent user"| poster
    core -->|"post as agent user"| poster
    growth -->|"post as agent user"| poster
    poster -->|"comments, issues,<br/>assignments"| issues

    post_gitea -->|"gtr comment"| issues
```

## Reviewer Chain (Mermaid)

```mermaid
%%{init: {'theme': 'base'}}%%
sequenceDiagram
    participant Issue as Gitea Issue
    participant IS as implementation-swarm
    participant QC as quality-coordinator
    participant TG as test-guardian
    participant SS as security-sentinel
    participant SV as spec-validator
    participant CW as compliance-watchdog
    participant MC as merge-coordinator

    Note over Issue: Finding filed or<br/>manual trigger

    Issue->>IS: @adf:implementation-swarm
    activate IS
    IS->>Issue: Branch + implement + PR
    IS->>Issue: @adf:quality-coordinator
    deactivate IS

    Issue->>QC: @adf:quality-coordinator
    activate QC
    QC->>Issue: compound-review verdict: GO
    QC->>Issue: @adf:test-guardian<br/>@adf:security-sentinel<br/>@adf:spec-validator<br/>@adf:compliance-watchdog
    deactivate QC

    par Four reviewers in parallel
        Issue->>TG: @adf:test-guardian
        activate TG
        TG->>Issue: test-guardian verdict: PASS/FAIL<br/>@adf:merge-coordinator
        deactivate TG
    and
        Issue->>SS: @adf:security-sentinel
        activate SS
        SS->>Issue: security-sentinel verdict: PASS/FAIL<br/>@adf:merge-coordinator
        deactivate SS
    and
        Issue->>SV: @adf:spec-validator
        activate SV
        SV->>Issue: spec-validator verdict: PASS/FAIL<br/>@adf:merge-coordinator
        deactivate SV
    and
        Issue->>CW: @adf:compliance-watchdog
        activate CW
        CW->>Issue: compliance-watchdog verdict: PASS/FAIL<br/>@adf:merge-coordinator
        deactivate CW
    end

    Issue->>MC: @adf:merge-coordinator (triggered multiple times)
    activate MC

    alt All 5 PASS
        MC->>Issue: Merge PR + close issue
    else Any FAIL
        MC->>Issue: Create [Remediation] issues per FAIL
        MC->>Issue: @adf:implementation-swarm on each
        Note over MC,IS: Cycle repeats for<br/>remediation issues
    else Any MISSING
        MC->>Issue: "Cannot merge" -- wait
    end
    deactivate MC
```

## Agent Token Coverage

```
Agent                    Token  Gitea User  Layer
------------------------ ------ ----------- --------
security-sentinel        yes    yes         Safety
drift-detector           yes    yes         Safety
test-guardian            yes    yes         Core
compliance-watchdog      yes    yes         Core
spec-validator           yes    yes         Core
implementation-swarm     yes    yes         Core
upstream-synchronizer    yes    yes         Core
meta-coordinator         yes    yes         Core
documentation-generator  yes    yes         Core
product-development      yes    yes         Core
quality-coordinator      yes    yes         Growth
merge-coordinator        yes    yes         Growth
browser-qa               yes    yes         Growth
---
analyse-findings         n/a    n/a         Flow step (uses root)
cargo-audit              n/a    n/a         Flow step (action)
check-findings           n/a    n/a         Flow step (gate)
post-to-gitea            n/a    n/a         Flow step (action)
```
