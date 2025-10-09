class AgentConfigManager {
    constructor(options) {
        this.apiClient = options.apiClient;
        this.roleSelector = document.getElementById(options.roleSelectorId);
        this.systemPrompt = document.getElementById(options.systemPromptId);
        this.onStateChange = options.onStateChange || (() => {});
        this.roles = {};
        this.lastSavedState = null; // Track last saved state to detect changes

        if (!this.roleSelector || !this.systemPrompt) {
            throw new Error('AgentConfigManager: UI elements not found.');
        }

        this.roleSelector.addEventListener('change', () => this.onRoleChange());
        this.systemPrompt.addEventListener('input', () => {
            this.onStateChange();
            this.debouncedSaveToBackend();
        });
    }

    async initialize() {
        if (!this.apiClient) {
            console.error('AgentConfigManager: ApiClient not provided.');
            return;
        }
        try {
            const response = await this.apiClient.getConfig();
            if (response && response.config && response.config.roles) {
                this.roles = response.config.roles;
                this.populateRoleSelector();
            }
        } catch (error) {
            console.error('Failed to load roles:', error);
        }
    }

    populateRoleSelector() {
        this.roleSelector.innerHTML = '';
        for (const roleName in this.roles) {
            const option = document.createElement('option');
            option.value = roleName;
            option.textContent = this.roles[roleName].name || roleName;
            this.roleSelector.appendChild(option);
        }
        this.onRoleChange();
    }

    onRoleChange() {
        const selectedRoleName = this.roleSelector.value;
        const role = this.roles[selectedRoleName];

        if (role && role.extra && role.extra.llm_system_prompt) {
            this.systemPrompt.value = role.extra.llm_system_prompt;
        } else {
            this.systemPrompt.value = 'This role has no default system prompt. You can define one here.';
        }
        this.onStateChange();
        this.saveAgentConfigToBackend();
    }

    applyState(state) {
        // Temporarily disable backend saving during state loading
        const originalOnStateChange = this.onStateChange;
        this.onStateChange = () => {}; // Disable state change callbacks
        
        if (state.selectedRole && this.roles[state.selectedRole]) {
            this.roleSelector.value = state.selectedRole;
            // Manually set the system prompt without triggering events
            const role = this.roles[state.selectedRole];
            if (role && role.extra && role.extra.llm_system_prompt) {
                this.systemPrompt.value = role.extra.llm_system_prompt;
            } else {
                this.systemPrompt.value = 'This role has no default system prompt. You can define one here.';
            }
        }
        
        // We only set the system prompt from state if it's different from the role default
        // This handles the case where a user has customized the prompt.
        const role = this.roles[this.roleSelector.value];
        const defaultPrompt = (role && role.extra && role.extra.llm_system_prompt) 
            ? role.extra.llm_system_prompt 
            : 'This role has no default system prompt. You can define one here.';
            
        if (state.systemPrompt && state.systemPrompt !== defaultPrompt) {
            this.systemPrompt.value = state.systemPrompt;
        }
        
        // Restore the original state change callback
        this.onStateChange = originalOnStateChange;
        
        // Update the last saved state to match what we just loaded
        this.lastSavedState = this.getState();
    }

    getState() {
        return {
            selectedRole: this.roleSelector.value,
            systemPrompt: this.systemPrompt.value,
        };
    }

    // Debounced save to backend to avoid too many API calls
    debouncedSaveToBackend() {
        if (this.saveTimeout) {
            clearTimeout(this.saveTimeout);
        }
        this.saveTimeout = setTimeout(() => {
            this.saveAgentConfigToBackend();
        }, 1000); // Save after 1 second of inactivity
    }

    // Save agent configuration to backend
    async saveAgentConfigToBackend() {
        if (!this.apiClient) {
            console.warn('AgentConfigManager: No API client available for saving to backend');
            return;
        }

        const currentState = this.getState();
        
        // Only save if state has actually changed
        if (this.lastSavedState && 
            this.lastSavedState.selectedRole === currentState.selectedRole &&
            this.lastSavedState.systemPrompt === currentState.systemPrompt) {
            return;
        }

        try {
            // Get the current config first
            const currentConfig = await this.apiClient.getConfig();
            
            if (currentConfig && currentConfig.config) {
                const roleName = currentState.selectedRole;
                const role = this.roles[roleName];
                
                if (role) {
                    // Create updated role configuration
                    const updatedRole = {
                        ...role,
                        extra: {
                            ...role.extra,
                            system_prompt: currentState.systemPrompt
                        }
                    };

                    // Update the complete config with the modified role
                    const configUpdate = {
                        ...currentConfig.config,
                        roles: {
                            ...currentConfig.config.roles,
                            [roleName]: updatedRole
                        }
                    };

                    await this.apiClient.updateConfig(configUpdate);
                    this.lastSavedState = { ...currentState };
                    console.log('Agent configuration saved to backend:', roleName);
                }
            }
        } catch (error) {
            console.error('Failed to save agent configuration to backend:', error);
        }
    }
}
