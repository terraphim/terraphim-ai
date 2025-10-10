# 🔥 TruthForge - Two-Pass Debate Arena

**Pattern**: Two-pass adversarial debate for crisis communication vulnerability analysis  
**Use Case**: Identify narrative weaknesses through systematic omission detection and exploitation  
**Key Features**: BiasDetector, NarrativeMapper, OmissionDetector, TaxonomyLinker, Exploitation Debate

## 🎯 Overview

TruthForge implements a sophisticated two-pass debate system designed to stress-test crisis communications and identify vulnerabilities before they become public relations disasters. The system uses adversarial AI agents to simulate both supportive and critical perspectives, with a specialized focus on detecting and exploiting omissions in crisis narratives.

## 🔄 Workflow Pattern

```
┌───────────────────────────────────────────────────────────────┐
│ PASS 1: Initial Balanced Debate                               │
│ ┌─────────────┐  ┌─────────────┐  ┌─────────────┐            │
│ │BiasDetector │  │NarrativeMap │  │Omission     │            │
│ │Cognitive    │  │Structure    │  │Detector     │            │
│ │Biases       │  │Claims       │  │Gaps Found   │            │
│ └─────────────┘  └─────────────┘  └─────────────┘            │
└───────────────────────────────────────────────────────────────┘
                <i class="fas fa-arrow-down"></i>
                 Omissions Feed Pass 2
                <i class="fas fa-arrow-down"></i>
┌───────────────────────────────────────────────────────────────┐
│ PASS 2: Exploitation Debate                                   │
│ ┌──────────────────────┐    ┌──────────────────────┐         │
│ │Supporting Agent      │    │Opposing Agent        │         │
│ │<i class="fas fa-shield-alt"></i> Defensive      │    │<i class="fas fa-bullseye"></i> Offensive       │         │
│ │Justifies omissions   │ VS │Exploits weaknesses   │         │
│ │Contextualizes gaps   │    │Constructs counter    │         │
│ └──────────────────────┘    └──────────────────────┘         │
│             <i class="fas fa-fire"></i> Vulnerability Assessment             │
└───────────────────────────────────────────────────────────────┘
```

## ✨ Key Features

### Pass 1: Balanced Analysis

**BiasDetector** 🔍
- Identifies cognitive biases in narrative framing
- Detects emotional manipulation patterns
- Highlights logical fallacies and rhetorical techniques

**NarrativeMapper** 🗺️
- Maps logical structure of claims
- Identifies causal relationships
- Builds argument dependency graph

**OmissionDetector** 👁️
- Systematically identifies missing information
- Categorizes omissions by severity
- Flags critical gaps in stakeholder communication

**TaxonomyLinker** 🔗
- Links narrative to established crisis communication frameworks
- Maps to industry best practices
- Identifies standard vs. non-standard disclosure patterns

### Pass 2: Adversarial Exploitation

**Supporting Agent (Defensive)** 🛡️
- Provides contextual justifications for detected omissions
- Constructs defensive narratives
- Addresses weaknesses with mitigation strategies

**Opposing Agent (Offensive)** 🎯
- Weaponizes omissions to construct adversarial counter-narratives
- Identifies maximum-impact exploitation vectors
- Simulates worst-case public criticism scenarios

**Vulnerability Assessment** 🔥
- Calculates defensive strength vs. attack effectiveness
- Assigns overall vulnerability rating (LOW/MEDIUM/HIGH)
- Provides actionable recommendations for narrative improvement

## 🚀 Getting Started

### Quick Start (5 minutes)

1. **Open the Interface**: Navigate to `index.html` in your browser
2. **Enter Crisis Narrative**: Paste or type your crisis communication text
3. **Configure Analysis**: Select depth (Standard/Deep/Rapid)
4. **Start Analysis**: Click "Start Analysis" button
5. **Review Results**: Examine Pass 1 and Pass 2 outputs

### Example Crisis Narrative

```
Our company experienced a data breach affecting 10,000 customer records. 
We discovered unauthorized access to our customer database on March 15th 
due to an unpatched security vulnerability in our legacy system. We have 
notified affected customers and are offering free credit monitoring services 
for 12 months.
```

**Expected Omissions**:
- When was the breach actually discovered vs. when it occurred?
- What specific data was compromised?
- How long did unauthorized access persist?
- Why wasn't the vulnerability patched earlier?
- Are there regulatory implications (GDPR, CCPA)?
- What happened to the attacker?

## 📊 Understanding Results

### Pass 1 Metrics

**Supporting Confidence (%)**: How well the narrative supports its claims with evidence  
**Opposing Confidence (%)**: How strongly the narrative can be challenged  
**Omissions Count**: Number of critical information gaps detected

### Pass 2 Metrics

**Defensive Strength (%)**: Effectiveness of justifications for omissions  
**Attack Effectiveness (%)**: Impact of exploitation attempts  
**Vulnerability Level**: Overall narrative robustness assessment

### Vulnerability Levels

| Level | Score Diff | Meaning | Action Required |
|-------|-----------|---------|-----------------|
| **LOW** | <10% | Narrative is robust | Minor refinements |
| **MEDIUM** | 10-30% | Moderate vulnerabilities | Address key omissions |
| **HIGH** | >30% | Significant weaknesses | Major narrative revision |

## 🔧 Technical Architecture

### API Integration

**Endpoint**: `POST /api/v1/truthforge`

**Request**:
```json
{
  "text": "Crisis narrative content..."
}
```

**Response**:
```json
{
  "status": "success",
  "session_id": "uuid-string",
  "analysis_url": "/api/v1/truthforge/session_id"
}
```

**Polling**: System polls analysis_url every 2 seconds for status updates

### Status Flow

1. `initiating` - Analysis starting
2. `pass1_in_progress` - BiasDetector, NarrativeMapper, OmissionDetector running
3. `pass1_complete` - Pass 1 results available, Pass 2 starting
4. `pass2_in_progress` - Exploitation debate underway
5. `complete` - Both passes complete, full results ready
6. `error` - Analysis failed with error message

### Data Structure

```javascript
{
  pass1: {
    bias_analysis: "Cognitive biases detected...",
    narrative_structure: "Claim mapping...",
    omissions: ["Missing detail 1", "Missing detail 2", ...],
    supporting_confidence: 72,
    opposing_confidence: 68
  },
  pass2: {
    supporting_argument: "Defensive justification...",
    opposing_argument: "Adversarial exploitation...",
    supporting_strength: 58,
    opposing_effectiveness: 84,
    vulnerability_level: "HIGH"
  }
}
```

## 🎨 UI Components

### Font Awesome Icons

**Pass 1 Icons**:
- `fa-comment-dots` - Initial Debate
- `fa-balance-scale` - BiasDetector
- `fa-map` - NarrativeMapper
- `fa-eye-slash` - OmissionDetector
- `fa-thumbs-up` - Supporting Confidence
- `fa-thumbs-down` - Opposing Confidence

**Pass 2 Icons**:
- `fa-crosshairs` - Exploitation Mode
- `fa-shield-alt` - Defensive Strength
- `fa-bullseye` - Attack Effectiveness
- `fa-fire` - Vulnerability Level

### Color Scheme

**Pass 1**: Blue gradient (`#dbeafe` to `#e0e7ff`) - Analytical, balanced  
**Pass 2**: Orange gradient (`#fef3c7` to `#fed7aa`) - Adversarial, high-stakes  
**Vulnerability HIGH**: Red (`#dc2626`) - Critical attention needed  
**Vulnerability MEDIUM**: Orange (`#f59e0b`) - Moderate concern  
**Vulnerability LOW**: Green (`#10b981`) - Narrative robust

## 💡 Best Practices

### Crafting Effective Crisis Narratives

1. **Be Specific**: Include dates, numbers, affected parties
2. **Show Causality**: Explain how the crisis occurred
3. **Address Stakeholders**: Consider all affected parties
4. **Demonstrate Action**: Show concrete steps taken
5. **Maintain Transparency**: Don't hide uncomfortable truths

### Interpreting Omission Patterns

**High-Severity Omissions**:
- Missing regulatory compliance information
- Unclear timeline of events
- Vague accountability statements
- No mention of ongoing risks

**Medium-Severity Omissions**:
- Limited technical details
- Incomplete stakeholder communication
- Partial remediation plans

**Low-Severity Omissions**:
- Background context
- Historical precedents
- Future prevention specifics

## 🔍 Advanced Usage

### Analysis Depth Settings

**Standard** (Default): Balanced speed and thoroughness  
**Deep Analysis**: Comprehensive examination, slower processing  
**Rapid Assessment**: Quick vulnerability scan, reduced detail

### Taxonomy Linking

When enabled, links narrative elements to:
- Crisis communication best practices (SCCT framework)
- Industry-specific disclosure standards
- Regulatory compliance requirements
- Public relations case studies

## 📚 Real-World Applications

### Use Cases

**Corporate Communications**:
- Pre-release review of crisis statements
- Media response preparation
- Stakeholder communication optimization

**Public Relations**:
- Crisis scenario planning
- Reputation risk assessment
- Message testing and refinement

**Legal/Compliance**:
- Regulatory disclosure review
- Liability exposure identification
- Documentation gap analysis

**Government/NGO**:
- Public statement vetting
- Emergency communication testing
- Transparency audit

## 🐛 Troubleshooting

### Common Issues

**"Analysis Failed" Error**:
- Check network connectivity to API server
- Verify server is running on correct port (default: 8090)
- Ensure narrative text is not empty

**Incomplete Results**:
- Refresh page and retry analysis
- Check browser console for JavaScript errors
- Verify API server logs for backend issues

**Slow Analysis**:
- Deep analysis mode takes longer (30-60 seconds)
- Large narratives (>2000 words) require additional processing
- Check server load and model response times

## 🔗 Related Resources

- [Two-Pass Debate Arena PRD](/home/alex/projects/zestic-at/trueforge/truthforge-ai/docs/PRD_TwoPassDebateArena.md)
- [TruthForge Backend Documentation](../../../crates/terraphim_truthforge/)
- [Agent Workflow Patterns](../README.md)
- [Crisis Communication Framework](https://en.wikipedia.org/wiki/Crisis_communication)

## 📝 Future Enhancements

- Real-time collaboration for team analysis
- Historical narrative comparison
- Automated improvement suggestions
- Industry-specific taxonomy modules
- Multi-language support
- PDF/Word export of analysis results

---

**Start analyzing crisis narratives today!** This tool helps organizations identify and address communication vulnerabilities before they become public relations crises.

*TruthForge: Forging stronger narratives through adversarial analysis.*
