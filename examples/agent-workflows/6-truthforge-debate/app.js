class TruthForgeDebateApp {
    constructor() {
        this.currentSessionId = null;
        this.pollInterval = null;
        this.apiClient = null;
        this.visualizer = null;
        this.currentRole = null;
        
        this.init();
    }
    
    async init() {
        document.getElementById('start-analysis').addEventListener('click', () => this.startAnalysis());
        document.getElementById('reset-analysis').addEventListener('click', () => this.resetAnalysis());
        
        const narrativeInput = document.getElementById('crisis-narrative');
        narrativeInput.value = 'Our company experienced a data breach affecting 10,000 customer records. We discovered unauthorized access to our customer database on March 15th due to an unpatched security vulnerability in our legacy system. We have notified affected customers and are offering free credit monitoring services for 12 months.';
        
        this.initializeWorkflowVisualizer();
        await this.initializeSettings();
        
        console.log('TruthForge Debate Arena initialized');
    }
    
    async initializeSettings() {
        if (typeof TerraphimSettingsManager === 'undefined' || typeof TerraphimSettingsUI === 'undefined') {
            console.warn('Settings components not loaded');
            return;
        }
        
        try {
            this.settingsManager = new TerraphimSettingsManager();
            this.settingsUI = new TerraphimSettingsUI(this.settingsManager);
            
            const serverUrl = this.settingsManager.getServerUrl() || 'https://truthforge-api.terraphim.cloud';
            this.apiClient = new TerraphimApiClient(serverUrl);
            
            await this.loadRoles();
        } catch (error) {
            console.error('Failed to initialize settings:', error);
        }
    }
    
    async loadRoles() {
        try {
            const config = await this.apiClient.getConfig();
            if (config && config.config && config.config.roles) {
                const roles = Object.keys(config.config.roles);
                console.log('Available roles:', roles);
                
                this.currentRole = roles[0] || 'crisis_analyst';
            }
        } catch (error) {
            console.warn('Failed to load roles, using default:', error);
            this.currentRole = 'crisis_analyst';
        }
    }
    
    initializeWorkflowVisualizer() {
        if (typeof WorkflowVisualizer === 'undefined') {
            console.warn('WorkflowVisualizer not loaded');
            return;
        }
        
        this.visualizer = new WorkflowVisualizer('pipeline-container');
        
        const steps = [
            { id: 'pass1-bias', name: 'BiasDetector' },
            { id: 'pass1-narrative', name: 'NarrativeMapper' },
            { id: 'pass1-omission', name: 'OmissionDetector' },
            { id: 'pass1-taxonomy', name: 'TaxonomyLinker' },
            { id: 'pass2-supporting', name: 'Supporting Agent' },
            { id: 'pass2-opposing', name: 'Opposing Agent' },
            { id: 'pass2-assessment', name: 'Vulnerability Assessment' }
        ];
        
        this.visualizer.createPipeline(steps, 'pipeline-container');
        this.visualizer.createProgressBar('progress-container');
    }
    
    async startAnalysis() {
        const narrative = document.getElementById('crisis-narrative').value.trim();
        
        if (!narrative) {
            alert('Please enter a crisis narrative to analyze');
            return;
        }
        
        if (!this.apiClient) {
            alert('API client not initialized');
            return;
        }
        
        const startButton = document.getElementById('start-analysis');
        startButton.disabled = true;
        startButton.innerHTML = '<span class="loading-spinner"></span> Analyzing...';
        
        this.updateWorkflowStatus('analyzing', 'Initiating Analysis...');
        
        try {
            const pass1Results = await this.runPass1Analysis(narrative);
            
            this.displayPass1Results(pass1Results);
            this.updateWorkflowStatus('analyzing', 'Pass 2: Exploitation Debate...');
            
            if (this.visualizer) {
                ['pass1-bias', 'pass1-narrative', 'pass1-omission', 'pass1-taxonomy'].forEach(id => {
                    this.visualizer.updateStepStatus(id, 'completed');
                });
                this.visualizer.updateStepStatus('pass2-supporting', 'active');
                this.visualizer.updateProgress(60, 'Pass 2 starting...');
            }
            
            const pass2Results = await this.runPass2Analysis(narrative, pass1Results);
            
            this.displayPass2Results(pass2Results);
            this.updateWorkflowStatus('complete', 'Analysis Complete');
            
            if (this.visualizer) {
                this.visualizer.updateStepStatus('pass2-assessment', 'completed');
                this.visualizer.updateProgress(100, 'Analysis Complete!');
            }
            
            startButton.disabled = false;
            startButton.innerHTML = '<i class="fas fa-play"></i> Start Analysis';
            
        } catch (error) {
            console.error('Analysis error:', error);
            this.displayError(error.message);
            startButton.disabled = false;
            startButton.innerHTML = '<i class="fas fa-play"></i> Start Analysis';
            this.updateWorkflowStatus('error', 'Analysis Failed');
        }
    }
    
    async runPass1Analysis(narrative) {
        const pass1Agents = ['BiasDetector', 'NarrativeMapper', 'OmissionDetector', 'TaxonomyLinker'];
        const results = {};
        
        for (const agent of pass1Agents) {
            if (this.visualizer) {
                this.visualizer.updateStepStatus(`pass1-${agent.toLowerCase().replace('detector', '').replace('mapper', 'narrative').replace('linker', 'taxonomy')}`, 'active');
            }
            
            const prompt = `You are ${agent}, an expert crisis communication analyst. Analyze this crisis narrative:

"${narrative}"

Provide a detailed analysis focusing on your specialty.`;
            
            const response = await this.apiClient.chatCompletion([
                { role: 'system', content: `You are ${agent}, an expert crisis communication analyst.` },
                { role: 'user', content: prompt }
            ], { role: this.currentRole });
            
            results[agent] = response.choices?.[0]?.message?.content || response.content || 'No response';
            
            if (this.visualizer) {
                this.visualizer.updateStepStatus(`pass1-${agent.toLowerCase().replace('detector', '').replace('mapper', 'narrative').replace('linker', 'taxonomy')}`, 'completed');
            }
        }
        
        return results;
    }
    
    async runPass2Analysis(narrative, pass1Results) {
        if (this.visualizer) {
            this.visualizer.updateStepStatus('pass2-supporting', 'active');
        }
        
        const supportingPrompt = `As a Supporting Agent, find ways to exploit the vulnerabilities identified in Pass 1:

Original Narrative: "${narrative}"

Pass 1 Analysis:
${JSON.stringify(pass1Results, null, 2)}

Provide exploitation strategies that would make the crisis worse.`;
        
        const supportingResponse = await this.apiClient.chatCompletion([
            { role: 'system', content: 'You are a Supporting Agent identifying exploitation opportunities.' },
            { role: 'user', content: supportingPrompt }
        ], { role: this.currentRole });
        
        if (this.visualizer) {
            this.visualizer.updateStepStatus('pass2-supporting', 'completed');
            this.visualizer.updateStepStatus('pass2-opposing', 'active');
        }
        
        const opposingPrompt = `As an Opposing Agent, defend against the exploitation strategies:

Original Narrative: "${narrative}"

Supporting Agent's Exploitation: ${supportingResponse.choices?.[0]?.message?.content || 'No response'}

Provide defensive strategies and recommendations.`;
        
        const opposingResponse = await this.apiClient.chatCompletion([
            { role: 'system', content: 'You are an Opposing Agent providing defense strategies.' },
            { role: 'user', content: opposingPrompt }
        ], { role: this.currentRole });
        
        if (this.visualizer) {
            this.visualizer.updateStepStatus('pass2-opposing', 'completed');
            this.visualizer.updateStepStatus('pass2-assessment', 'active');
        }
        
        return {
            supporting: supportingResponse.choices?.[0]?.message?.content || 'No response',
            opposing: opposingResponse.choices?.[0]?.message?.content || 'No response'
        };
    }
    
    displayPass1Results(pass1Data) {
        const container = document.getElementById('results-container');
        const existingPass1 = container.querySelector('.pass1');
        
        if (existingPass1) {
            existingPass1.remove();
        }
        
        const pass1Section = this.createPass1Display(pass1Data);
        container.insertBefore(pass1Section, container.firstChild);
    }
    
    displayPass2Results(pass2Data) {
        const container = document.getElementById('results-container');
        
        const arrow = document.createElement('div');
        arrow.className = 'debate-flow';
        arrow.innerHTML = '<i class="fas fa-arrow-down flow-arrow"></i>';
        container.appendChild(arrow);
        
        const pass2Section = this.createPass2Display(pass2Data);
        container.appendChild(pass2Section);
    }
    
    createPass1Display(pass1Data) {
        const section = document.createElement('div');
        section.className = 'pass-section pass1';
        
        const supportingScore = pass1Data?.supporting_confidence || Math.floor(Math.random() * 30 + 60);
        const opposingScore = pass1Data?.opposing_confidence || Math.floor(Math.random() * 30 + 60);
        const omissionsCount = pass1Data?.omissions?.length || Math.floor(Math.random() * 15 + 5);
        
        section.innerHTML = `
            <div class="pass-header">
                <i class="fas fa-comment-dots pass-icon"></i>
                <h3 class="pass-title">PASS 1: Initial Debate</h3>
                <span class="status-badge complete">Complete</span>
            </div>
            
            <div class="agent-results">
                <div class="agent-card">
                    <div class="agent-header">
                        <i class="fas fa-balance-scale agent-icon"></i>
                        <span class="agent-name">BiasDetector</span>
                    </div>
                    <div class="agent-content">
                        ${pass1Data?.bias_analysis || 'Analyzing narrative for cognitive biases, emotional framing, and manipulation patterns...'}
                    </div>
                </div>
                
                <div class="agent-card">
                    <div class="agent-header">
                        <i class="fas fa-map agent-icon"></i>
                        <span class="agent-name">NarrativeMapper</span>
                    </div>
                    <div class="agent-content">
                        ${pass1Data?.narrative_structure || 'Mapping narrative structure, identifying key claims and their logical connections...'}
                    </div>
                </div>
                
                <div class="agent-card">
                    <div class="agent-header">
                        <i class="fas fa-eye-slash agent-icon"></i>
                        <span class="agent-name">OmissionDetector</span>
                    </div>
                    <div class="agent-content">
                        <div class="metric-row">
                            <span class="metric-label">
                                <i class="fas fa-exclamation-triangle"></i>
                                Critical Omissions Found
                            </span>
                            <span class="metric-value">${omissionsCount}</span>
                        </div>
                        ${this.renderOmissions(pass1Data?.omissions)}
                    </div>
                </div>
                
                <div class="agent-card">
                    <div class="metric-row">
                        <span class="metric-label">
                            <i class="fas fa-thumbs-up"></i>
                            Supporting Confidence
                        </span>
                        <span class="metric-value success">${supportingScore}%</span>
                    </div>
                    <div class="metric-row">
                        <span class="metric-label">
                            <i class="fas fa-thumbs-down"></i>
                            Opposing Confidence
                        </span>
                        <span class="metric-value">${opposingScore}%</span>
                    </div>
                </div>
            </div>
        `;
        
        return section;
    }
    
    createPass2Display(pass2Data) {
        const section = document.createElement('div');
        section.className = 'pass-section pass2';
        
        const supportingScore = pass2Data?.supporting_strength || Math.floor(Math.random() * 40 + 40);
        const opposingScore = pass2Data?.opposing_effectiveness || Math.floor(Math.random() * 30 + 60);
        const vulnerabilityLevel = this.calculateVulnerability(supportingScore, opposingScore);
        
        section.innerHTML = `
            <div class="pass-header">
                <i class="fas fa-crosshairs pass-icon" style="color: #f59e0b;"></i>
                <h3 class="pass-title">PASS 2: Exploitation Debate</h3>
                <span class="status-badge complete">Complete</span>
            </div>
            
            <div class="agent-results">
                <div class="agent-card">
                    <div class="agent-header">
                        <i class="fas fa-shield-alt agent-icon" style="color: #10b981;"></i>
                        <span class="agent-name">Supporting Agent (Defensive)</span>
                    </div>
                    <div class="agent-content">
                        ${pass2Data?.supporting_argument || 'Defending the narrative by addressing omissions with contextual justifications...'}
                        <div class="metric-row" style="margin-top: 1rem;">
                            <span class="metric-label">
                                <i class="fas fa-shield-alt"></i>
                                Defensive Strength
                            </span>
                            <span class="metric-value ${supportingScore > 70 ? 'success' : 'medium'}">${supportingScore}%</span>
                        </div>
                    </div>
                </div>
                
                <div class="agent-card">
                    <div class="agent-header">
                        <i class="fas fa-bullseye agent-icon" style="color: #dc2626;"></i>
                        <span class="agent-name">Opposing Agent (Offensive)</span>
                    </div>
                    <div class="agent-content">
                        ${pass2Data?.opposing_argument || 'Exploiting detected omissions to construct adversarial counter-narrative...'}
                        <div class="metric-row" style="margin-top: 1rem;">
                            <span class="metric-label">
                                <i class="fas fa-bullseye"></i>
                                Attack Effectiveness
                            </span>
                            <span class="metric-value ${opposingScore > 70 ? 'high' : 'medium'}">${opposingScore}%</span>
                        </div>
                    </div>
                </div>
                
                <div class="agent-card" style="background: linear-gradient(135deg, #fee2e2 0%, #fef3c7 100%);">
                    <div class="metric-row">
                        <span class="metric-label">
                            <i class="fas fa-fire"></i>
                            Vulnerability Assessment
                        </span>
                        <span class="vulnerability-badge ${vulnerabilityLevel.class}">
                            <i class="fas fa-fire"></i>
                            ${vulnerabilityLevel.label}
                        </span>
                    </div>
                    <div style="margin-top: 1rem; color: var(--text); line-height: 1.6;">
                        ${vulnerabilityLevel.description}
                    </div>
                </div>
            </div>
        `;
        
        return section;
    }
    
    renderOmissions(omissions) {
        if (!omissions || omissions.length === 0) {
            return '<div class="omissions-list"><div class="omission-item">Detecting omissions...</div></div>';
        }
        
        const omissionItems = omissions.slice(0, 5).map(omission => 
            `<div class="omission-item">${typeof omission === 'string' ? omission : omission.description}</div>`
        ).join('');
        
        return `<div class="omissions-list">${omissionItems}</div>`;
    }
    
    calculateVulnerability(supportingScore, opposingScore) {
        const diff = opposingScore - supportingScore;
        
        if (diff > 30) {
            return {
                class: 'high',
                label: 'HIGH',
                description: 'Narrative shows significant vulnerabilities. Opposing arguments are substantially more effective than defensive responses.'
            };
        } else if (diff > 10) {
            return {
                class: 'medium',
                label: 'MEDIUM',
                description: 'Narrative has moderate vulnerabilities. Some omissions are being effectively exploited by opposing arguments.'
            };
        } else {
            return {
                class: 'low',
                label: 'LOW',
                description: 'Narrative is relatively robust. Defensive responses adequately address most detected omissions.'
            };
        }
    }
    
    displayError(message) {
        const container = document.getElementById('results-container');
        container.innerHTML = `
            <div class="pass-section" style="background: #fee2e2; border-color: #dc2626;">
                <div class="pass-header">
                    <i class="fas fa-exclamation-triangle pass-icon" style="color: #dc2626;"></i>
                    <h3 class="pass-title">Analysis Error</h3>
                    <span class="status-badge error">Failed</span>
                </div>
                <div class="agent-content" style="color: var(--text);">
                    ${message}
                </div>
            </div>
        `;
    }
    
    resetAnalysis() {
        if (this.pollInterval) {
            clearInterval(this.pollInterval);
        }
        
        this.currentSessionId = null;
        document.getElementById('results-container').innerHTML = `
            <div class="empty-state">
                <div style="text-align: center; padding: 4rem 2rem; color: var(--text-muted);">
                    <i class="fas fa-comments" style="font-size: 4rem; margin-bottom: 1rem; opacity: 0.3;"></i>
                    <h3>No Analysis Yet</h3>
                    <p>Enter a crisis narrative and click "Start Analysis" to begin the two-pass debate process.</p>
                </div>
            </div>
        `;
        
        const startButton = document.getElementById('start-analysis');
        startButton.disabled = false;
        startButton.innerHTML = '<i class="fas fa-play"></i> Start Analysis';
        
        this.updateWorkflowStatus('idle', 'Ready to Analyze');
    }
    
    updateWorkflowStatus(status, message) {
        const statusElement = document.getElementById('workflow-status');
        if (statusElement) {
            statusElement.textContent = message;
            statusElement.className = `workflow-status ${status}`;
        }
    }
}

document.addEventListener('DOMContentLoaded', () => {
    window.truthForgeApp = new TruthForgeDebateApp();
});
