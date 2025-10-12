/**
 * TruthForge UI - Crisis Communication Vulnerability Analysis
 * Standalone application for TruthForge Two-Pass Debate workflow
 */

class TruthForgeClient {
  constructor(baseUrl = 'http://localhost:8090') {
    this.baseUrl = baseUrl;
    this.currentSessionId = null;
    this.analysisResult = null;
    this.wsClient = null;
    this.startTime = null;

    this.initializeWebSocket();
  }

  initializeWebSocket() {
    if (typeof TerraphimWebSocketClient === 'undefined') {
      console.warn('WebSocket client not available, progress updates disabled');
      return;
    }

    const wsUrl = this.baseUrl.replace(/^http/, 'ws') + '/ws';

    try {
      this.wsClient = new TerraphimWebSocketClient({
        url: wsUrl,
        reconnectInterval: 3000,
        maxReconnectAttempts: 10
      });

      this.wsClient.subscribe('connected', () => {
        this.updateServerStatus('connected');
      });

      this.wsClient.subscribe('disconnected', () => {
        this.updateServerStatus('disconnected');
      });

      this.wsClient.subscribe('truthforge_progress', (message) => {
        if (message.session_id === this.currentSessionId) {
          this.handleProgressUpdate(message.data);
        }
      });
    } catch (error) {
      console.error('Failed to initialize WebSocket:', error);
      this.updateServerStatus('error');
    }
  }

  updateServerStatus(status) {
    const indicator = document.getElementById('serverStatus');
    const text = document.querySelector('.status-text');

    if (!indicator || !text) return;

    indicator.className = 'status-indicator';

    switch (status) {
      case 'connected':
        indicator.classList.add('connected');
        text.textContent = 'Connected';
        break;
      case 'disconnected':
        indicator.classList.add('disconnected');
        text.textContent = 'Disconnected';
        break;
      case 'error':
        indicator.classList.add('error');
        text.textContent = 'Connection Error';
        break;
      default:
        text.textContent = 'Connecting...';
    }
  }

  async submitNarrative(narrativeInput) {
    const response = await fetch(`${this.baseUrl}/api/v1/truthforge`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(narrativeInput)
    });

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }

    return await response.json();
  }

  async getAnalysis(sessionId) {
    const response = await fetch(`${this.baseUrl}/api/v1/truthforge/${sessionId}`);

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`);
    }

    return await response.json();
  }

  async pollForResults(sessionId, maxWaitSeconds = 120) {
    const pollInterval = 2000; // 2 seconds
    const maxAttempts = (maxWaitSeconds * 1000) / pollInterval;

    for (let attempt = 0; attempt < maxAttempts; attempt++) {
      await new Promise(resolve => setTimeout(resolve, pollInterval));

      const response = await this.getAnalysis(sessionId);

      if (response.result) {
        return response.result;
      }

      if (response.error) {
        throw new Error(`Analysis failed: ${response.error}`);
      }
    }

    throw new Error('Analysis timeout - results not ready after ' + maxWaitSeconds + ' seconds');
  }

  handleProgressUpdate(data) {
    const { stage, details } = data;

    switch (stage) {
      case 'started':
        this.updatePipelineStage('pass_one', 'omissions', 'running');
        this.updateProgressText(`Analysis started (${details.narrative_length} characters)`);
        break;

      case 'completed':
        this.markAllStagesComplete();
        this.updateProgressText(
          `Analysis complete! Found ${details.omissions_count} omissions, ` +
          `generated ${details.strategies_count} strategies (${details.processing_time_ms}ms)`
        );
        break;

      case 'failed':
        this.markAllStagesFailed();
        this.updateProgressText(`Analysis failed: ${details.error}`, 'error');
        break;
    }
  }

  updatePipelineStage(stage, step, status) {
    const stepElement = document.querySelector(
      `.pipeline-stage[data-stage="${stage}"] .step[data-step="${step}"] .step-status`
    );

    if (!stepElement) return;

    stepElement.className = 'step-status';

    switch (status) {
      case 'running':
        stepElement.classList.add('running');
        stepElement.textContent = '⏳';
        break;
      case 'complete':
        stepElement.classList.add('complete');
        stepElement.textContent = '✅';
        break;
      case 'error':
        stepElement.classList.add('error');
        stepElement.textContent = '❌';
        break;
    }
  }

  markAllStagesComplete() {
    const steps = document.querySelectorAll('.step .step-status');
    steps.forEach(step => {
      step.className = 'step-status complete';
      step.textContent = '✅';
    });
  }

  markAllStagesFailed() {
    const steps = document.querySelectorAll('.step .step-status');
    steps.forEach(step => {
      step.className = 'step-status error';
      step.textContent = '❌';
    });
  }

  updateProgressText(text, type = 'info') {
    const progressText = document.getElementById('progressText');
    if (!progressText) return;

    progressText.textContent = text;
    progressText.className = 'progress-text ' + type;

    // Update elapsed time
    if (this.startTime) {
      const elapsed = Math.floor((Date.now() - this.startTime) / 1000);
      const progressTime = document.getElementById('progressTime');
      if (progressTime) {
        progressTime.textContent = `${elapsed}s elapsed`;
      }
    }
  }
}

// UI Controller
class TruthForgeUI {
  constructor() {
    this.client = new TruthForgeClient();
    this.currentDebateView = 'pass1';

    this.initializeEventListeners();
    this.updateCharCount();
  }

  initializeEventListeners() {
    // Character count
    const textarea = document.getElementById('narrativeText');
    textarea?.addEventListener('input', () => this.updateCharCount());

    // Analyze button
    const analyzeBtn = document.getElementById('analyzeBtn');
    analyzeBtn?.addEventListener('click', () => this.handleAnalyze());

    // New analysis button
    const newAnalysisBtn = document.getElementById('newAnalysisBtn');
    newAnalysisBtn?.addEventListener('click', () => this.resetToInput());

    // Export button
    const exportBtn = document.getElementById('exportBtn');
    exportBtn?.addEventListener('click', () => this.exportResults());

    // Tab switching
    const tabButtons = document.querySelectorAll('.tab-btn');
    tabButtons.forEach(btn => {
      btn.addEventListener('click', (e) => this.switchTab(e.target.dataset.tab));
    });

    // Debate view toggle
    const debateToggles = document.querySelectorAll('.debate-toggle');
    debateToggles.forEach(toggle => {
      toggle.addEventListener('click', (e) => this.switchDebateView(e.target.dataset.debate));
    });
  }

  updateCharCount() {
    const textarea = document.getElementById('narrativeText');
    const charCount = document.getElementById('charCount');

    if (textarea && charCount) {
      charCount.textContent = textarea.value.length;
    }
  }

  async handleAnalyze() {
    const textarea = document.getElementById('narrativeText');
    const text = textarea?.value.trim();

    if (!text) {
      alert('Please enter a narrative to analyze');
      return;
    }

    // Gather form data
    const narrativeInput = {
      text: text,
      urgency: document.querySelector('input[name="urgency"]:checked')?.value || 'Low',
      stakes: Array.from(document.querySelectorAll('input[name="stakes"]:checked'))
        .map(cb => cb.value),
      audience: document.querySelector('input[name="audience"]:checked')?.value || 'Internal'
    };

    // Validate stakes
    if (narrativeInput.stakes.length === 0) {
      narrativeInput.stakes = ['Reputational'];
    }

    try {
      // Show pipeline section
      this.showSection('pipelineSection');
      this.hideSection('inputSection');

      // Disable analyze button
      const analyzeBtn = document.getElementById('analyzeBtn');
      if (analyzeBtn) {
        analyzeBtn.disabled = true;
        analyzeBtn.textContent = 'Analyzing...';
      }

      // Submit narrative
      this.client.startTime = Date.now();
      const response = await this.client.submitNarrative(narrativeInput);

      this.client.currentSessionId = response.session_id;

      // Update session info
      document.getElementById('sessionId').textContent = response.session_id;

      // Wait for results (either from WebSocket updates or polling)
      const result = await this.client.pollForResults(response.session_id);

      this.client.analysisResult = result;
      this.displayResults(result);

    } catch (error) {
      console.error('Analysis failed:', error);
      alert(`Analysis failed: ${error.message}`);
      this.resetToInput();
    }
  }

  displayResults(result) {
    // Show results section
    this.showSection('resultsSection');

    // Update session info
    const processingTime = document.getElementById('processingTime');
    if (processingTime && result.processing_time_ms) {
      processingTime.textContent = `${(result.processing_time_ms / 1000).toFixed(2)}s`;
    }

    const totalRisk = document.getElementById('totalRisk');
    if (totalRisk && result.omission_catalog) {
      const score = result.omission_catalog.total_risk_score;
      totalRisk.textContent = score.toFixed(2);
      totalRisk.className = 'info-value risk-score ' + this.getRiskClass(score);
    }

    // Display content in tabs
    this.displaySummary(result.executive_summary);
    this.displayOmissions(result.omission_catalog);
    this.displayDebate(result.pass_one_debate, result.pass_two_debate);
    this.displayVulnerability(result.cumulative_analysis);
    this.displayStrategies(result.response_strategies);
  }

  displaySummary(summary) {
    const summaryContent = document.getElementById('summaryContent');
    if (!summaryContent) return;

    summaryContent.innerHTML = `
      <div class="summary-text">
        ${this.formatMarkdown(summary)}
      </div>
    `;
  }

  displayOmissions(catalog) {
    const omissionsList = document.getElementById('omissionsList');
    if (!omissionsList || !catalog) return;

    const omissions = catalog.omissions || [];

    if (omissions.length === 0) {
      omissionsList.innerHTML = '<p class="empty-state">No omissions detected</p>';
      return;
    }

    omissionsList.innerHTML = omissions.map((om, idx) => `
      <div class="omission-card">
        <div class="omission-header">
          <span class="omission-number">#${idx + 1}</span>
          <span class="omission-category">${this.formatCategory(om.category)}</span>
          <span class="risk-badge ${this.getRiskClass(om.composite_risk)}">
            Risk: ${(om.composite_risk * 100).toFixed(0)}%
          </span>
        </div>
        <p class="omission-description">${om.description}</p>
        <div class="omission-metrics">
          <div class="metric">
            <span class="metric-label">Severity:</span>
            <span class="metric-value">${(om.severity * 100).toFixed(0)}%</span>
          </div>
          <div class="metric">
            <span class="metric-label">Exploitability:</span>
            <span class="metric-value">${(om.exploitability * 100).toFixed(0)}%</span>
          </div>
        </div>
      </div>
    `).join('');
  }

  displayDebate(pass1, pass2) {
    // Initial view is Pass 1
    this.renderDebateTranscript(pass1, 'pass1');
    this.storedDebates = { pass1, pass2 };
  }

  switchDebateView(debateType) {
    this.currentDebateView = debateType;

    // Update active toggle
    document.querySelectorAll('.debate-toggle').forEach(toggle => {
      toggle.classList.toggle('active', toggle.dataset.debate === debateType);
    });

    // Render appropriate debate
    const debate = debateType === 'pass1' ? this.storedDebates.pass1 : this.storedDebates.pass2;
    this.renderDebateTranscript(debate, debateType);
  }

  renderDebateTranscript(debate, type) {
    const transcript = document.getElementById('debateTranscript');
    if (!transcript || !debate) return;

    const isPass2 = type === 'pass2';

    transcript.innerHTML = `
      <div class="debate-messages">
        <div class="message supporting">
          <div class="message-header">
            <span class="speaker">${isPass2 ? '🛡️ Defensive' : '👍 Supporting'}</span>
            <span class="score">Score: ${debate.supporting_score?.toFixed(2) || 'N/A'}</span>
          </div>
          <div class="message-content">
            ${this.formatMarkdown(debate.supporting_argument?.main_argument || '')}
          </div>
        </div>

        <div class="message opposing">
          <div class="message-header">
            <span class="speaker">${isPass2 ? '⚔️ Exploitation' : '👎 Opposing'}</span>
            <span class="score">Score: ${debate.opposing_score?.toFixed(2) || 'N/A'}</span>
          </div>
          <div class="message-content">
            ${this.formatMarkdown(debate.opposing_argument?.main_argument || '')}
          </div>
        </div>

        <div class="message evaluator">
          <div class="message-header">
            <span class="speaker">⚖️ Evaluator</span>
            <span class="winner">${debate.winning_position || 'TBD'}</span>
          </div>
          <div class="message-content">
            ${this.formatMarkdown(debate.evaluation?.reasoning || '')}
          </div>
        </div>
      </div>
    `;
  }

  displayVulnerability(analysis) {
    const vulnerabilityAnalysis = document.getElementById('vulnerabilityAnalysis');
    if (!vulnerabilityAnalysis || !analysis) return;

    vulnerabilityAnalysis.innerHTML = `
      <div class="vulnerability-metrics">
        <div class="metric-card">
          <h4>Strategic Risk Level</h4>
          <div class="risk-level ${this.getRiskClass(analysis.vulnerability_delta || 0)}">
            ${analysis.strategic_risk_level || 'Unknown'}
          </div>
        </div>

        <div class="metric-card">
          <h4>Vulnerability Delta</h4>
          <div class="delta-value">
            ${((analysis.vulnerability_delta || 0) * 100).toFixed(1)}%
          </div>
          <p class="delta-desc">Increase in vulnerability from Pass 1 to Pass 2</p>
        </div>

        <div class="metric-card">
          <h4>Amplification Factor</h4>
          <div class="amplification-value">
            ${(analysis.amplification_factor || 1).toFixed(2)}x
          </div>
          <p class="delta-desc">Opposition strength multiplier</p>
        </div>
      </div>

      <div class="recommended-actions">
        <h4>Recommended Actions</h4>
        <ul>
          ${(analysis.recommended_actions || []).map(action =>
            `<li>${action}</li>`
          ).join('')}
        </ul>
      </div>

      ${analysis.point_of_failure ? `
        <div class="point-of-failure">
          <h4>⚠️ Point of Failure</h4>
          <p>${analysis.point_of_failure}</p>
        </div>
      ` : ''}
    `;
  }

  displayStrategies(strategies) {
    const strategiesList = document.getElementById('strategiesList');
    if (!strategiesList || !strategies) return;

    strategiesList.innerHTML = strategies.map(strategy => `
      <div class="strategy-card">
        <div class="strategy-header">
          <h3>${this.formatStrategyType(strategy.strategy_type)}</h3>
          <span class="strategy-risk ${this.getRiskClass(strategy.risk_assessment?.media_amplification || 0)}">
            Media Risk: ${((strategy.risk_assessment?.media_amplification || 0) * 100).toFixed(0)}%
          </span>
        </div>

        <div class="strategy-rationale">
          <h4>Strategic Rationale</h4>
          <p>${strategy.strategic_rationale}</p>
        </div>

        <div class="drafts-container">
          ${strategy.drafts.social_media ? `
            <div class="draft-section">
              <h4>📱 Social Media</h4>
              <div class="draft-content">${this.formatMarkdown(strategy.drafts.social_media)}</div>
              <button class="copy-btn" onclick="ui.copyToClipboard(this, \`${this.escapeQuotes(strategy.drafts.social_media)}\`)">
                Copy
              </button>
            </div>
          ` : ''}

          ${strategy.drafts.press_statement ? `
            <div class="draft-section">
              <h4>📰 Press Statement</h4>
              <div class="draft-content">${this.formatMarkdown(strategy.drafts.press_statement)}</div>
              <button class="copy-btn" onclick="ui.copyToClipboard(this, \`${this.escapeQuotes(strategy.drafts.press_statement)}\`)">
                Copy
              </button>
            </div>
          ` : ''}

          ${strategy.drafts.internal_memo ? `
            <div class="draft-section">
              <h4>📝 Internal Memo</h4>
              <div class="draft-content">${this.formatMarkdown(strategy.drafts.internal_memo)}</div>
              <button class="copy-btn" onclick="ui.copyToClipboard(this, \`${this.escapeQuotes(strategy.drafts.internal_memo)}\`)">
                Copy
              </button>
            </div>
          ` : ''}
        </div>
      </div>
    `).join('');
  }

  // Utility methods
  formatMarkdown(text) {
    if (!text) return '';

    // Simple markdown rendering
    return text
      .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
      .replace(/\*(.*?)\*/g, '<em>$1</em>')
      .replace(/\n/g, '<br>');
  }

  formatCategory(category) {
    const map = {
      'MissingEvidence': 'Missing Evidence',
      'UnstatedAssumption': 'Unstated Assumption',
      'AbsentStakeholder': 'Absent Stakeholder',
      'ContextGap': 'Context Gap',
      'UnaddressedCounterargument': 'Unaddressed Counterargument'
    };
    return map[category] || category;
  }

  formatStrategyType(type) {
    const map = {
      'Reframe': '🔄 Reframe Strategy',
      'CounterArgue': '🎯 Counter-Argue Strategy',
      'Bridge': '🌉 Bridge Strategy'
    };
    return map[type] || type;
  }

  getRiskClass(score) {
    if (score >= 0.7) return 'severe';
    if (score >= 0.5) return 'high';
    if (score >= 0.3) return 'moderate';
    return 'low';
  }

  escapeQuotes(text) {
    return text.replace(/`/g, '\\`').replace(/\$/g, '\\$');
  }

  copyToClipboard(button, text) {
    navigator.clipboard.writeText(text).then(() => {
      const original = button.textContent;
      button.textContent = '✓ Copied!';
      setTimeout(() => {
        button.textContent = original;
      }, 2000);
    });
  }

  switchTab(tabName) {
    // Update tab buttons
    document.querySelectorAll('.tab-btn').forEach(btn => {
      btn.classList.toggle('active', btn.dataset.tab === tabName);
    });

    // Update tab panels
    document.querySelectorAll('.tab-panel').forEach(panel => {
      panel.classList.toggle('active', panel.dataset.panel === tabName);
    });
  }

  showSection(sectionId) {
    const section = document.getElementById(sectionId);
    if (section) {
      section.classList.remove('hidden');
    }
  }

  hideSection(sectionId) {
    const section = document.getElementById(sectionId);
    if (section) {
      section.classList.add('hidden');
    }
  }

  resetToInput() {
    this.hideSection('pipelineSection');
    this.hideSection('resultsSection');
    this.showSection('inputSection');

    // Reset analyze button
    const analyzeBtn = document.getElementById('analyzeBtn');
    if (analyzeBtn) {
      analyzeBtn.disabled = false;
      analyzeBtn.innerHTML = '<span class="btn-icon">🚀</span> Analyze Narrative';
    }

    // Clear form
    const textarea = document.getElementById('narrativeText');
    if (textarea) {
      textarea.value = '';
      this.updateCharCount();
    }
  }

  exportResults() {
    if (!this.client.analysisResult) {
      alert('No results to export');
      return;
    }

    const data = JSON.stringify(this.client.analysisResult, null, 2);
    const blob = new Blob([data], { type: 'application/json' });
    const url = URL.createObjectURL(blob);

    const a = document.createElement('a');
    a.href = url;
    a.download = `truthforge-${this.client.currentSessionId}.json`;
    a.click();

    URL.revokeObjectURL(url);
  }
}

// Initialize UI when DOM is ready
let ui;
document.addEventListener('DOMContentLoaded', () => {
  ui = new TruthForgeUI();
});
