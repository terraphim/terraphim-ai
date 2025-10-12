# User journey and product design concept for TruthForge AI

[TruthForge taxonomy.rtf](https://t90151159089.p.clickup-attachments.com/t90151159089/886ea442-6c69-4559-98cd-25fb0900af2f/TruthForge%20taxonomy.rtf)

## 🌍 Overall User Journey (PR Manager in TruthForge AI)
### 1\. **Intake & Context Setting**
*   **Action:** PR manager pastes in a narrative, news article, or social media post.
*   **System Prompt:** “Set urgency and context.”
    *   High urgency (immediate response)
    *   High reputational/legal stakes
    *   Public/media facing vs. internal/stakeholder facing
👉 _Taxonomy link:_
*   _Issue & Crisis Management → assess\_and\_classify_
*   _Strategic Management → scan\_environment_
* * *
### 2\. **AI Narrative Analysis**
*   **Agents at work:**
    *   _Bias Detector:_ highlights loaded words, selective framing, hidden disqualification tactics.
    *   _Narrative Mapper:_ extracts stakeholder frames, attribution of responsibility, and alignment with issue/crisis typologies.
*   **Output:** “Narrative X positions you as \[preventable/accidental/victim\] in SCCT terms; here are hidden frames.”
👉 _Taxonomy link:_
*   _Issue & Crisis → risk\_assessment, attribution\_map_
*   _Relationship Management → listening\_infrastructure_
* * *
### 3\. **AI Debate Simulation**
*   **Action:** Two agents take opposing sides on the narrative; third agent evaluates.
*   **System Insight:**
    *   Shows weak points, exploitable contradictions, missing evidence.
    *   Maps out where stakeholders would perceive high risk.
*   **Output UI:** “Argument Scorecard” → Strengths, Vulnerabilities, Bias Flags.
👉 _Taxonomy link:_
*   _Issue & Crisis → war\_room\_operations_
*   _Strategic Management → executive\_advisory, risk\_reputation\_briefs_
* * *
### 4\. **Counterframe Construction**
*   **Action:** PR manager chooses between:
    *   **Reframe** (change context, open dialogue, reduce polarization).
    *   **Counter-argue** (direct rebuttal grounded in facts).
    *   **Bridge** (pivot to shared values or long-term commitments).
*   **Output:** Drafted statements, Q&A sets, narrative reframing templates.
👉 _Taxonomy link:_
*   _Relationship Management → dialogue\_playbooks, advocacy\_programs_
*   _Issue & Crisis → holding\_statements, SCCT\_response\_matrix_
* * *
### 5\. **Decision & Activation**
*   **Action:** User selects final option:
    *   Deploy (social media/press release templates, CRM push).
    *   Escalate (legal, board briefing).
    *   Store (scenario library for future training).
*   **Output:** Publishing dashboard with channels and governance guardrails.
👉 _Taxonomy link:_
*   _Strategic Management → comms\_objectives\_OKRs, governance\_raci_
*   _Issue & Crisis → single\_source\_of\_truth\_portal, approval\_workflows_
* * *
### 6\. **Learning & Feedback**
*   **System provides:**
    *   Engagement metrics (response time, sentiment shift, trust delta).
    *   Relationship dashboards (advocacy rate, stakeholder quality index).
    *   Crisis playbook updates.
👉 _Taxonomy link:_
*   _Relationship Management → relationship\_quality\_index_
*   _Strategic Management → integrated\_comms\_scorecards_
*   _Issue & Crisis → after\_action\_review_
* * *
## 🎨 Product Design Concept
**Design Metaphor:** _“War Room meets Debate Stage”_
*   **Home Screen:** “Narrative Forge” – input box, urgency toggle, context selector.
*   **Analysis Panel:** Real-time heatmap of bias, framing, and attribution.
*   **Debate Arena:** Animated AI agents debating the issue; evaluator agent gives live “argument health scores.”
*   **Counterframe Studio:** Modular templates (fact rebuttal, reframing, bridging) with preview for each channel.
*   **Activation Dashboard:** One-click export to press kits, social media, CRM stakeholder logs.
*   **Governance Layer:** Approval workflows, legal sign-off, audit trail.
*   **Learning Vault:** Case archive, crisis playbook updates, relationship dashboards.
**Core UX principles:**
*   _Tactical urgency_ → fast, responsive workflows for crises.
*   _Strategic memory_ → building long-term institutional knowledge.
*   _Dialogic symmetry_ → enable two-way framing, not just one-way spin.
* * *
# 📱 TruthForge AI – Screen Wireframe Concept
### 1\. **Home / Narrative Intake**
*   **Header:** Logo + tagline (“Reclaim the Frame. Dominate the Debate.”)
*   **Main Panel:**
    *   Large **input text box** → “Paste narrative / article / post here”
    *   **Context & Priority selector** (radio or segmented buttons):
        *   High Urgency 🔥
        *   High Reputational/Legal Stakes ⚖️
        *   Media-Facing 🎤 / Internal 🏢
    *   **CTA Button:** “Forge Analysis”
![](https://t90151159089.p.clickup-attachments.com/t90151159089/9d9eb06d-25a7-4158-a9af-4b083bc08762/image.png)
* * *
### 2\. **Narrative Analysis Dashboard**
*   **Left Sidebar:** Timeline of active cases (like a “war room log”).
*   **Main Panel (3 tabs):**
    1. **Bias Scan** – highlighted text with color-coded flags (loaded words, framing, disqualification tactics).
    2. **Narrative Map** – stakeholder roles, SCCT classification (victim/accidental/preventable), framing radar.
    3. **Heatmap View** – sentiment / risk plotted over time or channels.
*   **Right Sidebar:** Key Insights box → “Top 3 vulnerabilities / strengths.”
![](https://t90151159089.p.clickup-attachments.com/t90151159089/ac22644e-7e21-4c1a-932a-d355534af77d/image.png)
* * *
### 3\. **Debate Arena**
*   **Visual Metaphor:** Split screen with **Agent A** vs. **Agent B**, speech bubbles animated as they argue.
*   **Below Debate:**
    *   Evaluator Agent’s **Scorecard**:
        *   Argument Strength meter
        *   Bias Exposure meter
        *   Exploitability alerts (e.g., “Weak on evidence”, “Over-reliant on moral framing”)
*   **Toggle:** “Show in plain text” vs. “Simulation mode.”
![](https://t90151159089.p.clickup-attachments.com/t90151159089/7390e6a8-e485-455c-8eb0-da0b9c517d82/image.png)
* * *
### 4\. **Counterframe Studio**
*   **Main Panel:**
    *   **Options Carousel**:
        *   Reframe 🌐 (bridge to shared values)
        *   Counter-argue ⚔️ (fact-based rebuttal)
        *   Bridge 🕊️ (pivot to future commitments)
    *   **Draft Workspace:** editable text area with AI-suggested draft.
    *   **Tone Selector:** Formal / Neutral / Empathetic / Assertive.
*   **Right Sidebar:** Channel Previews → Twitter/X, Press Release, Internal Memo.
![](https://t90151159089.p.clickup-attachments.com/t90151159089/10cca73c-2cdd-4be1-acd3-4ded64248f01/image.png)
* * *
### 5\. **Activation Dashboard**
*   **Top Bar:** Governance guardrails → “Legal Approval Needed” badge.
*   **Channel Cards (grid):**
    *   Press Release
    *   Social Media (X, LinkedIn, FB)
    *   CRM Stakeholder Log
    *   Internal Alert
*   Each card has **Preview + Send / Export** button.
*   **Audit Trail Panel:** Timestamp + user approvals.
![](https://t90151159089.p.clickup-attachments.com/t90151159089/8214b99b-f4b6-445c-8a1d-0157ea0406fe/image.png)
* * *
### 6\. **Learning Vault**
*   **Main Panel:**
    *   Archive of past cases → searchable by issue, crisis, stakeholder.
    *   **Metrics Dashboard:** sentiment shift, advocacy rate, reputation delta.
    *   **Playbook Updates:** evolving crisis scenarios and reframing strategies.
*   **Sidebar:** Personalized recommendations (“Train team on disqualification tactics detected in last 3 cases”).
* * *
# 🎨 Design Style Guide
*   **Visual Tone:** Tactical + Strategic → dark background, accent colors for urgency (red/orange), credibility (blue/teal), neutrality (gray).
*   **Metaphor:** _War Room meets Strategic Compass._
*   **Components:**
    *   Modular cards (for debates, insights, governance).
    *   Collapsible sidebars (case log / audit trail).
    *   Animated debate arena (optional playful but serious UI).
