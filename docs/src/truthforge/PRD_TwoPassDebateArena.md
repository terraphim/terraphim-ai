# Product Requirements Document: TruthForge Two-Pass Debate Arena

**Version**: 1.0
**Date**: 2025-10-07
**Status**: Draft
**Owner**: Zestic AI / K-Partners
**Linear Issues**: [ZES-12](https://linear.app/zestic-ai/issue/ZES-12), [ZES-11](https://linear.app/zestic-ai/issue/ZES-11)

---

## Executive Summary

TruthForge AI is a privacy-first, Rust-based narrative intelligence platform that helps PR professionals and crisis communication teams analyze contested narratives, identify vulnerabilities, and craft strategic responses. This PRD defines the **Two-Pass Debate Arena** feature - an advanced multi-agent workflow that simulates adversarial debate in two phases: initial analysis with omission detection (Pass 1), followed by exploitation-focused debate that targets identified gaps (Pass 2).

### Strategic Context

Built on the Terraphim-AI multi-agent framework, TruthForge leverages:
- **Role-based agents** with specialized expertise
- **Knowledge graph taxonomy** for strategic communication frameworks
- **Real-time WebSocket orchestration** for responsive user experience
- **Redis-backed persistence** for session management
- **OpenRouter LLM integration** for production-grade AI capabilities

This product will be delivered as a **private Rust crate** (`terraphim_truthforge`) integrating with the public terraphim-ai ecosystem.

---

## Product Vision

**Mission**: Empower communication professionals to reclaim narrative control through AI-powered strategic intelligence.

**Vision**: Become the go-to platform for high-stakes narrative analysis, enabling PR managers to:
1. Quickly identify bias, framing tactics, and hidden assumptions
2. Simulate adversarial debate to uncover vulnerabilities before opponents do
3. Craft evidence-based, strategically sound responses
4. Learn from each engagement to build institutional knowledge

**Market Position**: TruthForge positions as a **"Narrative Playground meets Debate Stage"** - combining tactical crisis response with strategic narrative planning.

---

## User Personas

### Primary: PR Manager (Crisis Communications)

**Profile**:
- **Role**: Senior PR/Communications professional at enterprise or agency
- **Experience**: 5-15 years in crisis management, media relations
- **Challenges**:
  - Tight deadlines during crisis situations
  - Need to anticipate opponent arguments before they emerge
  - Pressure to craft messages that withstand scrutiny
  - Limited time to consult with legal, C-suite, subject matter experts

**Goals**:
- Rapidly assess narrative risks and vulnerabilities
- Identify what's *not* being said (omissions, gaps, unstated assumptions)
- Simulate how adversaries will attack the organization's position
- Generate strategic response options with clear risk/benefit analysis

**Pain Points**:
- Existing tools are too slow for crisis response
- Manual analysis misses subtle framing tactics
- No way to "stress test" responses before publication
- Tools don't learn from past crises

### Secondary: Strategic Communication Consultant

**Profile**:
- **Role**: Independent consultant advising C-suite on reputation management
- **Experience**: 10-20 years across multiple industries and crisis types
- **Challenges**:
  - Clients expect immediate expert analysis
  - Need to demonstrate value through insight quality
  - Must stay current on evolving narrative tactics

**Goals**:
- Provide differentiated strategic intelligence
- Build playbooks for recurring crisis scenarios
- Demonstrate ROI through measurable outcomes

### Tertiary: Corporate Communications Team Lead

**Profile**:
- **Role**: Head of Communications at mid-large enterprise
- **Experience**: Managing team of 5-15 communication professionals
- **Challenges**:
  - Need to scale best practices across team
  - Building institutional memory around crisis response
  - Training junior team members on strategic thinking

**Goals**:
- Standardize narrative analysis workflows
- Build learning library from past engagements
- Enable team to handle complex scenarios independently

---

## User Journey: Six-Phase Workflow

TruthForge follows a comprehensive 6-phase user journey aligned with strategic communication best practices and Terraphim multi-agent workflow patterns.

### Phase 1: Intake & Context Setting

**User Action**: PR manager pastes contested narrative (news article, social media post, internal memo)

**System Behavior**:
- Accept text input (500-5000 words optimal)
- Present context configuration:
  - **Urgency**: High (immediate response) vs. Low (strategic planning)
  - **Stakes**: Reputational, legal, financial, operational
  - **Audience**: Public/media vs. internal stakeholders

**Terraphim Pattern**: Initial routing workflow (Pattern #2)
- Route to appropriate analysis depth based on context
- Complexity assessment determines agent allocation

**Output**: Validated narrative with context metadata

**Acceptance Criteria**:
- [ ] Accepts text input 100-10,000 words
- [ ] Context toggles update analysis parameters
- [ ] Invalid inputs show helpful error messages
- [ ] Context saved for session resumption

---

### Phase 2: AI Narrative Analysis

**User Action**: System automatically analyzes narrative upon submission

**System Behavior** - **Pass 1: Orchestrator-Workers Pattern**:

1. **BiasDetectorAgent** (content_critic role):
   - Identifies loaded language, selective framing
   - Flags disqualification tactics (ad hominem, guilt-by-association)
   - Detects logical fallacies and rhetorical devices
   - **Output**: Bias scorecard with specific text highlights

2. **NarrativeMapperAgent** (content_analyzer role):
   - Maps stakeholder roles and perspectives
   - Applies SCCT crisis classification (victim/accidental/preventable)
   - Identifies attribution of responsibility patterns
   - **Output**: Stakeholder map + SCCT classification

3. **TaxonomyLinkerAgent** (knowledge_mapper role):
   - Links narrative to TruthForge strategic communication taxonomy
   - Identifies applicable subfunctions (e.g., "horizon_scanning", "risk_assessment")
   - Maps to lifecycle stage (e.g., "assess_and_classify", "activate_and_respond")
   - **Output**: Taxonomy mapping with recommended playbooks

4. **OmissionDetectorAgent** (methodology_expert role) - **NEW**:
   - Identifies missing evidence and unstated assumptions
   - Detects absent stakeholder perspectives
   - Flags context gaps and unexplained decisions
   - Notes potential counterarguments not addressed
   - **Output**: Structured omission catalog with severity scores

**Terraphim Pattern**: Orchestrator-Workers (Pattern #4)
- Orchestrator agent coordinates parallel analysis
- Worker agents execute specialized analysis tasks
- Results aggregated for comprehensive view

**Real-time Updates**: WebSocket progress stream
- Each agent reports progress: 0% → 100%
- Visual pipeline shows which agents are active
- Estimated time remaining updates dynamically

**Output**: Comprehensive analysis dashboard with:
- Bias patterns and scores
- Stakeholder map and SCCT classification
- Taxonomy mapping and playbook recommendations
- **Omission catalog** (categorized gaps)

**Acceptance Criteria**:
- [ ] All 4 agents complete within 30 seconds (OpenRouter)
- [ ] WebSocket updates every 500ms
- [ ] Omission catalog identifies ≥5 distinct categories
- [ ] Results persist in Redis for 24 hours
- [ ] Each analysis includes confidence scores (0.0-1.0)

---

### Phase 3: AI Debate Simulation - Two-Pass Workflow

This is the **core innovation** of TruthForge - a two-pass debate that first identifies vulnerabilities, then simulates how adversaries would exploit them.

#### Pass 1: Initial Debate with Omission Awareness

**System Behavior**:

1. **DebaterAgent_Supporting** (creative_writer role):
   - Argues in favor of the narrative's legitimacy
   - Builds defensive messaging using analysis insights
   - **Aware of omissions** identified by OmissionDetectorAgent
   - Attempts to address gaps preemptively
   - **Output**: Supporting argument with evidence quality score

2. **DebaterAgent_Opposing** (content_critic role):
   - Challenges narrative validity and completeness
   - Identifies exploitable weaknesses
   - **Highlights omissions** as attack vectors
   - Questions attribution and stakeholder framing
   - **Output**: Opposing argument with attack surface analysis

3. **Pass1EvaluatorAgent** (content_editor role):
   - Scores both arguments on strength, evidence quality, coherence
   - Produces vulnerability scorecard
   - **Catalogs exploitable omissions** for Pass 2
   - Identifies which arguments would resonate with key stakeholders
   - **Output**: Debate scorecard + prioritized omission list for exploitation

**Terraphim Pattern**: Parallelization (Pattern #3) with evaluator
- Supporting and opposing agents run concurrently
- Evaluator synthesizes results with weighted scoring
- Consensus tracking on vulnerability areas

**Pass 1 Output**:
- Argument strength scores (supporting vs. opposing)
- Vulnerability heatmap (which narrative claims are weakest)
- **Prioritized omission catalog** (top 10 exploitable gaps)
- Stakeholder perception risk assessment

#### Pass 2: Exploitation-Focused Debate

**System Behavior**: Use Pass 1 omission catalog as explicit attack vectors

1. **ExploitationDebaterAgent_Supporting** (creative_writer + omission context):
   - Receives Pass 1 omission catalog
   - Attempts to defend narrative *despite* known gaps
   - Explores how to close omissions with additional context
   - **Output**: Defensive strategy acknowledging gaps

2. **ExploitationDebaterAgent_Opposing** (content_critic + omission context):
   - **Explicitly targets** top 10 omissions from Pass 1
   - Simulates adversarial tactics: "Why didn't they mention X?"
   - Constructs maximally damaging arguments using gap analysis
   - **Output**: Exploitation argument weaponizing omissions

3. **CumulativeEvaluatorAgent** (synthesis_specialist role) - **NEW**:
   - Compares Pass 1 vs Pass 2 argument strength
   - Tracks vulnerability amplification (how omissions compound under pressure)
   - Identifies "point of failure" - where narrative defense collapses
   - Produces strategic risk assessment
   - **Output**: Cumulative vulnerability analysis with recommended mitigations

**Terraphim Pattern**: Evaluator-Optimizer (Pattern #5)
- Pass 2 iteratively exploits weaknesses identified in Pass 1
- Evaluator measures vulnerability amplification
- Optimization loop continues until maximum exploitation identified

**Pass 2 Output**:
- Amplified vulnerability scores (how much worse under attack)
- Point of failure analysis (critical weaknesses)
- Strategic risk level (Low / Moderate / High / Severe)
- **Recommended defensive actions** to close exploitable gaps

**UI Visualization** (with Font Awesome Icons):
```
┌───────────────────────────────────────────────────────────────┐
│ <i class="fas fa-comment-dots"></i> PASS 1: Initial Debate   │
│ <i class="fas fa-thumbs-up"></i> Supporting: 72%             │
│ <i class="fas fa-thumbs-down"></i> Opposing: 68%             │
│ <i class="fas fa-eye-slash"></i> Omissions: 12 found         │
└───────────────────────────────────────────────────────────────┘
                <i class="fas fa-arrow-down"></i>
┌───────────────────────────────────────────────────────────────┐
│ <i class="fas fa-crosshairs"></i> PASS 2: Exploitation       │
│ <i class="fas fa-shield-alt"></i> Supporting: 58%            │
│ <i class="fas fa-bullseye"></i> Opposing: 84%                │
│ <i class="fas fa-fire"></i> Vulnerability: HIGH              │
└───────────────────────────────────────────────────────────────┘
```

**Acceptance Criteria**:
- [ ] Pass 1 completes in <20 seconds
- [ ] Pass 2 completes in <25 seconds (total <45s)
- [ ] Pass 2 arguments explicitly reference ≥80% of top 10 omissions
- [ ] Cumulative evaluator tracks vulnerability delta (Pass 2 - Pass 1)
- [ ] Results include specific text examples from both passes
- [ ] WebSocket streams both passes with visual differentiation

---

### Phase 4: Counterframe Construction

**User Action**: PR manager chooses strategic response approach based on debate results

**System Behavior**: Present 3 response strategies with AI-generated drafts

1. **Reframe** (shift context, reduce polarization):
   - ReframeAgent (content_editor role)
   - Focuses on shared values, dialogue opening
   - Pivots from defensive to constructive framing
   - **Output**: Reframing templates for social media, press, internal

2. **Counter-argue** (direct rebuttal with facts):
   - CounterArgueAgent (copy_editor role)
   - Fact-based corrections addressing specific claims
   - Closes gaps identified in omission analysis
   - **Output**: Q&A sets, fact sheets, rebuttal statements

3. **Bridge** (pivot to future commitments):
   - BridgeAgent (marketing_specialist role)
   - Acknowledges concerns, commits to action
   - Moves from conflict to collaboration
   - **Output**: Stakeholder letters, commitment statements

**Terraphim Pattern**: Parallelization (Pattern #3)
- All 3 response agents run concurrently
- User selects best-fit strategy based on context
- Each agent aware of debate vulnerabilities

**Output**: 3 response option packages with:
- Strategic rationale (why this approach fits the situation)
- Multi-channel drafts (social, press, internal)
- Risk assessment (what could go wrong)
- Tone guidance (formal, empathetic, assertive)

**Acceptance Criteria**:
- [ ] All 3 strategies generated in <15 seconds
- [ ] Each strategy addresses top 3 vulnerabilities from Pass 2
- [ ] Social media drafts ≤280 characters
- [ ] Press statements 2-3 paragraphs with clear structure
- [ ] Copy functionality works without app crash (ZES-11)

---

### Phase 5: Decision & Activation

**User Action**: Select final response strategy and deployment channels

**System Behavior**:
- Present activation dashboard with channel-specific formatting
- **Simplified UI** per ZES-11 requirements:
  - **Keep**: Executive briefing, key findings, playbooks, function classification
  - **Remove**: Technical metadata, duplicate headers, taxonomy details tab
  - **Fix**: Copy button must not crash app

**Deployment Channels**:
- Press release (formatted, ready to copy)
- Social media (Twitter/X, LinkedIn variants)
- Internal memo (for stakeholder communication)
- Q&A brief (for spokesperson prep)

**Governance Guardrails**:
- Legal review checklist
- Brand compliance check
- Regulatory disclosure requirements (if applicable)
- Approval workflow tracking

**Output**: Export package with:
- Selected response in all channel formats
- Governance checklist (completed/pending)
- Audit trail (timestamp, approvals, edits)
- Session ID for future reference

**Acceptance Criteria**:
- [ ] Export formats: JSON, Markdown, plain text
- [ ] Copy to clipboard works reliably (ZES-11)
- [ ] Governance checklist customizable per organization
- [ ] Audit trail persisted in Redis for 30 days
- [ ] Single-click export of complete package

---

### Phase 6: Learning & Feedback

**User Action**: System automatically captures performance data

**System Behavior**:
- Track engagement metrics (time to response, channels used)
- Measure sentiment shift (before/after deployment) - *future: social listening integration*
- Build case library of past analyses
- Update agent performance models

**Terraphim Integration**:
- `VersionedMemory`: Each agent learns from successful/failed analyses
- `VersionedTaskList`: Track common workflows and optimize
- `VersionedLessons`: Capture strategic insights for future crises

**Learning Vault Features**:
- Searchable archive of past debates
- Pattern recognition (similar crises, recurring omissions)
- Performance dashboards (agent accuracy, response quality)
- Playbook evolution (update crisis templates based on outcomes)

**Output**: Learning dashboard with:
- Case archive (searchable by issue type, stakeholder, SCCT classification)
- Agent performance metrics (accuracy, confidence calibration)
- Recommended playbook updates
- Emerging pattern alerts

**Acceptance Criteria**:
- [ ] Every session auto-saved to learning vault
- [ ] Search by narrative keywords, SCCT type, date range
- [ ] Agent performance tracked per role and phase
- [ ] Privacy controls (redact sensitive information before archiving)
- [ ] Export learning data for external analysis

---

## Functional Requirements

### FR-1: Narrative Input & Validation

**Description**: Accept and validate narrative text for analysis

**Requirements**:
- Support 100-10,000 word inputs
- Detect and handle non-English text (error message)
- Preserve formatting (paragraphs, headings) for context
- Auto-detect urgency indicators (keywords like "breaking", "urgent")

**Edge Cases**:
- Empty input → clear error message
- Extremely long input (>10k words) → offer to truncate or analyze in sections
- Special characters, emojis → handle gracefully without breaking analysis

---

### FR-2: Context Configuration

**Description**: Capture situational context to inform analysis depth

**Requirements**:
- **Urgency toggle**: High (immediate response) / Low (strategic planning)
  - High urgency → faster agents, shorter outputs
  - Low urgency → deeper analysis, more comprehensive

- **Stakes toggle**: Select applicable impacts
  - Reputational
  - Legal/regulatory
  - Financial
  - Operational
  - Social license

- **Audience toggle**: Public/media vs. Internal stakeholders
  - Public → emphasize media framing, external perception
  - Internal → focus on stakeholder alignment, governance

**Acceptance Criteria**:
- [ ] Toggles update analysis parameters in real-time
- [ ] Context saved with session for reproducibility
- [ ] Context influences agent system prompts
- [ ] Help text explains each toggle's impact

---

### FR-3: Real-Time Multi-Agent Orchestration

**Description**: Execute 6-phase workflow with live progress updates

**Requirements**:
- **WebSocket server** (Axum + tokio-tungstenite)
  - Client connects on session start
  - Server streams agent progress updates
  - Heartbeat every 30s to maintain connection

- **Agent coordination** (terraphim_multi_agent workflows)
  - Pass 1: 4 parallel agents (Bias, Narrative, Taxonomy, Omission)
  - Debate: Sequential passes with evaluation between
  - Pass 2: Exploitation agents using Pass 1 results
  - Response: 3 parallel strategy agents

- **Progress reporting**:
  ```json
  {
    "type": "agent_progress",
    "agent": "BiasDetectorAgent",
    "phase": "analysis",
    "status": "analyzing",
    "progress": 0.65,
    "estimated_remaining_ms": 8000
  }
  ```

**Performance Requirements**:
- Pass 1 analysis: <20 seconds (4 agents parallel)
- Pass 2 debate: <25 seconds (2 passes sequential)
- Response generation: <15 seconds (3 agents parallel)
- **Total workflow: <60 seconds** (target: 45 seconds)

**Acceptance Criteria**:
- [ ] WebSocket maintains connection for 5-minute sessions
- [ ] Progress updates every ≤500ms
- [ ] Agent failures don't crash entire workflow (graceful degradation)
- [ ] Connection drop → auto-reconnect with session ID

---

### FR-4: Omission Detection & Cataloging

**Description**: **NEW** agent that identifies gaps in narrative

**Requirements**:
- **OmissionDetectorAgent** analyzes for:
  1. **Missing Evidence**: Claims without supporting data
  2. **Unstated Assumptions**: Implied premises not explicitly stated
  3. **Absent Stakeholder Voices**: Perspectives not represented
  4. **Context Gaps**: Background information omitted
  5. **Unaddressed Counterarguments**: Obvious rebuttals ignored

- **Omission Structure**:
  ```json
  {
    "category": "missing_evidence",
    "description": "Claim about 40% reduction lacks data source",
    "severity": 0.85,
    "exploitability": 0.92,
    "text_reference": "We achieved 40% reduction in incidents"
  }
  ```

- **Prioritization**: Rank omissions by `exploitability` score
  - High exploitability (>0.8) → top priority for Pass 2
  - Severity × Exploitability = composite risk score

**Acceptance Criteria**:
- [ ] Detects ≥5 omission categories per analysis
- [ ] Each omission includes specific text reference
- [ ] Exploitability score based on narrative context (stakes, audience)
- [ ] Top 10 omissions passed to Pass 2 agents
- [ ] Omission catalog exportable as standalone report

---

### FR-5: Two-Pass Debate Simulation

**Description**: Core innovation - iterative debate revealing amplified vulnerabilities

**Pass 1 Requirements**:
- Supporting agent builds defense with omission awareness
- Opposing agent challenges narrative, highlights gaps
- Evaluator scores arguments + creates omission priority list

**Pass 2 Requirements**:
- Exploitation agents receive top 10 omissions from Pass 1
- Supporting agent defends *despite* known gaps
- Opposing agent weaponizes omissions in maximally damaging way
- Cumulative evaluator measures vulnerability amplification

**Evaluation Metrics**:
- **Argument Strength**: 0.0-1.0 (coherence, evidence quality, stakeholder resonance)
- **Vulnerability Delta**: (Pass 2 - Pass 1) shows amplification
- **Point of Failure**: Specific claim/omission where defense collapses
- **Strategic Risk**: Low / Moderate / High / Severe

**Acceptance Criteria**:
- [ ] Pass 2 arguments explicitly cite ≥80% of top 10 omissions
- [ ] Cumulative evaluator quantifies vulnerability delta
- [ ] Point of failure identified with confidence score
- [ ] Strategic risk level aligns with stakeholder perception data (future: validation)

---

### FR-6: Strategic Response Generation

**Description**: Generate 3 response strategies addressing vulnerabilities

**Requirements**:
- **ReframeAgent**: Shift context to reduce polarization
  - Find common ground, shared values
  - Recontextualize narrative to open dialogue
  - Avoid defensive posture

- **CounterArgueAgent**: Direct fact-based rebuttal
  - Address specific claims and omissions
  - Provide missing evidence
  - Correct the record professionally

- **BridgeAgent**: Pivot to future commitments
  - Acknowledge stakeholder concerns
  - Commit to specific actions
  - Build toward collaboration

**Output Formats**:
- Social media (≤280 characters)
- Press statement (2-3 paragraphs)
- Internal memo (1 page)
- Q&A brief (anticipated questions + answers)

**Acceptance Criteria**:
- [ ] All 3 strategies generated in <15 seconds
- [ ] Each addresses top 3 vulnerabilities from debate
- [ ] Tone matches context (formal for high stakes, empathetic for public)
- [ ] Outputs include strategic rationale and risk assessment

---

### FR-7: Simplified UI (ZES-11 Requirements)

**Description**: Clean, focused interface removing technical clutter

**KEEP**:
- ✅ Narrative input with context toggles
- ✅ Executive briefing (moved to TOP of results)
- ✅ Key findings from each phase
- ✅ Strategic taxonomy → "Playbooks to Use"
- ✅ Function classification (primary + secondary)
- ✅ Debate Arena visualization
- ✅ Copy button for analysis results

**REMOVE**:
- ❌ Technical agent metadata (execution time, confidence intervals)
- ❌ Taxonomy details tab
- ❌ Narrative mapping tab (consolidate into key findings)
- ❌ Duplicate headers ("Debate Stage Arena" appears twice)
- ❌ Download JSON button (copy provides same functionality)

**FIX**:
- 🔧 **Copy button MUST NOT crash app** (ZES-11 critical issue)
  - Use `navigator.clipboard.writeText()` with error handling
  - Fallback to `document.execCommand('copy')` for older browsers
  - Show success/failure toast notification

**UI Structure** (Agent-Workflows Pattern with Font Awesome Icons):
```
┌─────────────────────────────────────────────────────────────┐
│ 1. Narrative Input                                          │
│    <i class="fas fa-file-alt"></i> Paste text here...       │
│    <i class="fas fa-sliders-h"></i> Context Toggles         │
│    <i class="fas fa-play-circle"></i> [Analyze Button]      │
├─────────────────────────────────────────────────────────────┤
│ 2. Progress Pipeline (Real-time)                            │
│    <i class="fas fa-spinner fa-pulse"></i> Pass 1 →         │
│    <i class="fas fa-balance-scale"></i> Debate →            │
│    <i class="fas fa-crosshairs"></i> Pass 2 →               │
│    <i class="fas fa-lightbulb"></i> Response                │
├─────────────────────────────────────────────────────────────┤
│ 3. <i class="fas fa-star"></i> Executive Briefing (TOP)     │
│    One-paragraph strategic summary                          │
├─────────────────────────────────────────────────────────────┤
│ 4. <i class="fas fa-search"></i> Key Findings               │
│    • <i class="fas fa-exclamation-triangle"></i> Bias       │
│    • <i class="fas fa-tag"></i> SCCT classification         │
│    • <i class="fas fa-eye-slash"></i> Top 5 omissions       │
├─────────────────────────────────────────────────────────────┤
│ 5. <i class="fas fa-book"></i> Playbooks to Use             │
│    <i class="fas fa-shield-alt"></i> Issue & Crisis Mgmt    │
│    <i class="fas fa-tasks"></i> Risk Assessment, Response   │
├─────────────────────────────────────────────────────────────┤
│ 6. <i class="fas fa-comments"></i> Debate Arena             │
│    <i class="fas fa-chart-bar"></i> Pass 1 vs Pass 2        │
│    <i class="fas fa-fire"></i> Vulnerability heatmap        │
├─────────────────────────────────────────────────────────────┤
│ 7. <i class="fas fa-reply-all"></i> Strategic Responses     │
│    <i class="fas fa-sync-alt"></i> [Reframe]                │
│    <i class="fas fa-gavel"></i> [Counter-Argue]             │
│    <i class="fas fa-handshake"></i> [Bridge]                │
│    <i class="fas fa-copy"></i> Copy button ← MUST WORK      │
└─────────────────────────────────────────────────────────────┘
```

**Font Awesome Classic CDN**:
```html
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/font-awesome/6.5.1/css/all.min.css" />
```

**Acceptance Criteria**:
- [ ] Executive briefing visible without scrolling
- [ ] No technical jargon in main UI
- [ ] Copy button works 100% of time (no crashes)
- [ ] Removed elements don't leave visual gaps
- [ ] Mobile responsive (agent-workflows shared CSS)

---

### FR-8: Redis Persistence

**Description**: Session management and results caching

**Requirements**:
- **Session Storage** (TTL: 24 hours):
  ```
  Key: session:{uuid}
  Value: {narrative, context, timestamp, status}
  ```

- **Analysis Results** (TTL: 7 days):
  ```
  Key: results:{session_id}
  Value: {pass1, pass2, responses, metadata}
  ```

- **Learning Vault** (TTL: 90 days):
  ```
  Key: vault:case:{session_id}
  Value: {redacted_narrative, omissions, outcomes}
  ```

**Connection Handling**:
- Connection pool: 10 connections
- Retry strategy: exponential backoff (100ms → 3.2s)
- Fallback: In-memory cache if Redis unavailable
- Health check endpoint: `/health` returns Redis status

**Acceptance Criteria**:
- [ ] Session survives server restart
- [ ] Results retrievable by session ID
- [ ] TTL enforced (auto-cleanup)
- [ ] Graceful degradation if Redis down
- [ ] Privacy: PII redacted before vault storage

---

## Non-Functional Requirements

### NFR-1: Performance

- **Latency**:
  - Pass 1 analysis: <20 seconds (p95)
  - Pass 2 debate: <25 seconds (p95)
  - Total workflow: <60 seconds (p95), <45 seconds (target)

- **Throughput**:
  - Support 10 concurrent analyses (initial deployment)
  - Scale to 50 concurrent (6 months post-launch)

- **WebSocket**:
  - Progress updates every ≤500ms
  - Connection stable for 10-minute sessions

### NFR-2: Reliability

- **Uptime**: 99.5% (excluding planned maintenance)
- **Error Handling**:
  - Agent failures → graceful degradation (skip failed agent, continue workflow)
  - LLM timeout (30s) → retry once, then fail with clear message
  - Redis unavailable → in-memory fallback

- **Data Integrity**:
  - All results persisted before WebSocket close
  - Session recovery after connection drop

### NFR-3: Security

- **Authentication**: OAuth2 integration (future: SSO for enterprise)
- **Authorization**: Role-based access (Admin, PR Manager, Viewer)
- **Data Privacy**:
  - Narratives stored encrypted at rest (AES-256)
  - PII detection + redaction before learning vault storage
  - GDPR compliance: right to deletion (purge session data)

- **LLM Security**:
  - Prompt sanitization (prevent injection attacks)
  - Rate limiting: 100 requests/hour per user
  - Cost limits: max $5 per analysis (OpenRouter budget)

### NFR-4: Scalability

- **Horizontal Scaling**:
  - Stateless WebSocket servers (session in Redis)
  - Load balancer distributes connections

- **Agent Pool**:
  - Pre-warm agent pool (5 agents per role)
  - Auto-scale based on queue depth

- **Database**:
  - Redis cluster for high availability
  - Sharding by session ID prefix

### NFR-5: Maintainability

- **Code Quality**:
  - Rust: 80% test coverage
  - Clippy warnings = 0
  - Documentation for all public APIs

- **Observability**:
  - Structured logging (JSON format)
  - Metrics: Prometheus exporter
  - Tracing: OpenTelemetry integration

- **Deployment**:
  - Docker containers for all services
  - Kubernetes manifests for orchestration
  - CI/CD: GitHub Actions (test, build, deploy)

### NFR-6: Usability

- **Accessibility**: WCAG 2.1 Level AA compliance
- **Browser Support**: Chrome, Firefox, Safari, Edge (latest 2 versions)
- **Mobile**: Responsive design (agent-workflows CSS framework)
- **Error Messages**: Clear, actionable (avoid technical jargon)

---

## Success Metrics

### North Star Metric
**Time to Strategic Insight**: From narrative input to actionable response recommendation

**Target**: <60 seconds (currently: manual process takes 2-4 hours)

### Key Performance Indicators (KPIs)

#### Product Metrics
- **Adoption**: 50 active PR professionals in first 6 months
- **Engagement**: 80% of users complete full 6-phase workflow
- **Retention**: 60% monthly active users (MAU) return within 30 days
- **Net Promoter Score (NPS)**: >40

#### Quality Metrics
- **Omission Detection Accuracy**: ≥85% of omissions validated by expert review
- **Debate Realism**: ≥75% of users rate Pass 2 as "realistic adversary simulation"
- **Response Utility**: ≥70% of generated responses used (with or without edits)

#### Technical Metrics
- **Performance**: 95% of workflows complete in <60 seconds
- **Reliability**: <1% error rate across all phases
- **Availability**: 99.5% uptime
- **Copy Functionality**: 100% success rate (ZES-11)

#### Business Metrics
- **Customer Satisfaction (CSAT)**: >4.2/5.0
- **Cost per Analysis**: <$2 (OpenRouter LLM costs)
- **Time Saved**: 90% reduction vs. manual process
- **Revenue**: $50k ARR by end of Year 1 (enterprise tier at $5k/year)

---

## Competitive Analysis

### Existing Solutions

#### 1. Cision/Meltwater (Media Monitoring)
**Strengths**: Comprehensive media tracking, sentiment analysis
**Weaknesses**: Reactive (no predictive analysis), expensive ($10k-50k/year), no debate simulation
**TruthForge Advantage**: Proactive vulnerability detection, adversarial simulation, 10x cost reduction

#### 2. Critical Mention / Brandwatch
**Strengths**: Social listening, influencer tracking
**Weaknesses**: Surface-level analysis, no strategic response generation
**TruthForge Advantage**: Deep narrative analysis, multi-phase debate, strategic playbooks

#### 3. Manual Process (In-house PR teams)
**Strengths**: Tailored to organization, human expertise
**Weaknesses**: Slow (2-4 hours), inconsistent quality, no omission detection
**TruthForge Advantage**: 97% faster, systematic omission detection, institutional learning

#### 4. Generic AI Tools (ChatGPT, Claude)
**Strengths**: Accessible, low cost
**Weaknesses**: No specialized PR knowledge, no debate simulation, no systematic workflow
**TruthForge Advantage**: Purpose-built for crisis communication, validated against PR frameworks, two-pass analysis

### Differentiation Matrix

| Feature | TruthForge | Cision | ChatGPT | Manual |
|---------|------------|--------|---------|--------|
| Speed | <1 min | Hours | Minutes | 2-4 hrs |
| Omission Detection | ✅ Systematic | ❌ None | ⚠️ Inconsistent | ⚠️ Expert-dependent |
| Adversarial Simulation | ✅ Two-pass | ❌ None | ❌ None | ⚠️ Manual roleplay |
| SCCT Framework | ✅ Integrated | ❌ None | ❌ None | ⚠️ If consultant trained |
| Cost | $5k/yr | $25k/yr | $240/yr | $150k/yr (salary) |
| Learning Vault | ✅ Auto | ⚠️ Manual export | ❌ None | ⚠️ If documented |

---

## Risk Assessment

### Technical Risks

#### Risk: LLM Quality Variance
**Probability**: Medium
**Impact**: High
**Mitigation**:
- Use Claude 3.5 Sonnet (proven high quality) for critical agents
- Implement quality thresholds (reject analysis if confidence <0.7)
- Fallback to human review for high-stakes scenarios

#### Risk: WebSocket Connection Instability
**Probability**: Medium
**Impact**: Medium
**Mitigation**:
- Auto-reconnect with exponential backoff
- Session recovery via Redis persistence
- Fallback to REST polling if WebSocket fails

#### Risk: Redis Downtime
**Probability**: Low
**Impact**: Medium
**Mitigation**:
- Redis cluster with automatic failover
- In-memory cache fallback (loses persistence)
- Health check alerts before user impact

### Business Risks

#### Risk: Market Adoption (PR professionals skeptical of AI)
**Probability**: Medium
**Impact**: High
**Mitigation**:
- Pilot program with K-Partners clients (built-in user base)
- Emphasize AI as "augmentation not replacement"
- Provide expert validation of omission detection accuracy

#### Risk: Privacy Concerns (sensitive narratives)
**Probability**: Low
**Impact**: High
**Mitigation**:
- On-premise deployment option for enterprises
- PII redaction before storage
- GDPR compliance documentation
- Transparent data handling policies

#### Risk: Cost Overruns (LLM usage)
**Probability**: Medium
**Impact**: Medium
**Mitigation**:
- Cost caps per analysis ($5 max)
- Monitor token usage, optimize prompts
- Consider self-hosted LLMs for high-volume users

### Product Risks

#### Risk: Omission Detection False Positives
**Probability**: High (new feature)
**Impact**: Medium
**Mitigation**:
- Confidence scoring (only show omissions >0.7 confidence)
- User feedback loop ("Was this omission valid?")
- Continuous model fine-tuning based on feedback

#### Risk: Pass 2 Doesn't Amplify Vulnerabilities
**Probability**: Low
**Impact**: High
**Mitigation**:
- System prompt engineering (explicit instructions to exploit)
- Test with known vulnerable narratives
- Expert review of Pass 2 arguments during beta

---

## Go-to-Market Strategy

### Target Segments

**Tier 1 (Launch)**: K-Partners existing clients (warm leads)
- 20-30 enterprises with active PR/crisis needs
- Direct sales through K-Partners relationship
- Pilot pricing: $3k/year for first 10 users

**Tier 2 (6 months)**: PR agencies and consultancies
- 500+ agencies in US/UK market
- Inbound marketing: case studies, webinars
- Standard pricing: $5k/year per organization (unlimited users)

**Tier 3 (12 months)**: In-house corporate PR teams
- Fortune 500 communications departments
- Enterprise pricing: $25k/year + custom integrations
- SSO, on-premise deployment options

### Launch Timeline

**Month 1-2**: Private beta (K-Partners clients)
- 10 pilot users, intensive feedback
- Focus: omission detection accuracy, UI usability
- Goal: 80% CSAT, <5% error rate

**Month 3-4**: Public beta (PR agencies)
- 50 users, limited feature set
- Marketing: PR industry webinars, case studies
- Goal: 100 analyses run, validate business model

**Month 5-6**: General availability
- Full feature release, enterprise tier
- Pricing finalized based on beta learnings
- Goal: 50 paying customers, $50k ARR

### Marketing Channels

- **Content Marketing**: PR industry blog posts, omission detection guides
- **Partnerships**: Integration with Cision/Meltwater (complementary, not competitive)
- **Events**: PR conferences (PRSA, IABC), crisis communication workshops
- **SEO**: Target "narrative analysis", "crisis communication tools", "SCCT framework"

---

## Appendices

### Appendix A: SCCT Framework Integration

TruthForge integrates Situational Crisis Communication Theory (SCCT) classifications:

**Victim Cluster**:
- Natural disasters
- Product tampering (external)
- Workplace violence (external)
- **Response**: Instructing information, adjusting information, bolstering

**Accidental Cluster**:
- Technical errors
- Unintended consequences
- Stakeholder challenges
- **Response**: Instructing + adjusting + possible apology

**Preventable Cluster**:
- Organizational misconduct
- Management negligence
- Repeated failures
- **Response**: Full apology, compensation, corrective action

NarrativeMapperAgent classifies narratives into these clusters, guiding response strategy selection.

### Appendix B: Terraphim Workflow Pattern Mapping

| TruthForge Phase | Terraphim Pattern | Primary Workflow |
|------------------|-------------------|------------------|
| Intake & Context | Routing (Pattern #2) | Complexity-based agent allocation |
| Analysis (Pass 1) | Orchestrator-Workers (Pattern #4) | 4 parallel specialist agents |
| Debate (Pass 1) | Parallelization (Pattern #3) | Supporting + Opposing + Evaluator |
| Debate (Pass 2) | Evaluator-Optimizer (Pattern #5) | Iterative exploitation |
| Response Generation | Parallelization (Pattern #3) | 3 parallel strategy agents |
| Learning | Custom (agent_evolution) | Memory, tasks, lessons tracking |

### Appendix C: Agent Role Configuration Example

```json
{
  "name": "Omission Detector",
  "shortname": "omission_detector",
  "relevance_function": "BM25Plus",
  "extra": {
    "llm_provider": "openrouter",
    "llm_model": "anthropic/claude-3.5-sonnet",
    "system_prompt": "You are an expert at identifying gaps, missing context, and unstated assumptions in narratives. For each narrative, systematically analyze:\n\n1. Missing Evidence: Claims without supporting data, statistics, or sources\n2. Unstated Assumptions: Implied premises or beliefs not explicitly stated\n3. Absent Stakeholder Voices: Perspectives or groups not represented\n4. Context Gaps: Background information, history, or circumstances omitted\n5. Unaddressed Counterarguments: Obvious rebuttals or alternative explanations ignored\n\nFor each omission, provide:\n- Category (from list above)\n- Description (what's missing)\n- Severity (0.0-1.0, impact if exploited)\n- Exploitability (0.0-1.0, ease of attack)\n- Text Reference (specific quote triggering omission)\n\nPrioritize omissions by composite risk (severity × exploitability).",
    "agent_type": "omission_detector",
    "quality_criteria": ["completeness", "evidence_gaps", "stakeholder_coverage", "contextual_depth"],
    "taxonomy_mapping": "issue_crisis_management.risk_assessment",
    "max_tokens": 2000,
    "temperature": 0.3
  },
  "haystacks": []
}
```

### Appendix D: WebSocket Protocol Specification

See SPEC_TerraphimIntegration.md Section 4.3 for complete protocol details.

---

## Approval & Sign-off

**Product Owner**: __________________ Date: __________
**Engineering Lead**: __________________ Date: __________
**Design Lead**: __________________ Date: __________
**Stakeholder (K-Partners)**: __________________ Date: __________

---

**Document Control**
**Created**: 2025-10-07
**Last Modified**: 2025-10-07
**Version**: 1.0
**Classification**: Internal - Zestic AI Confidential
