<script lang="ts">
import { open } from '@tauri-apps/api/dialog';
import { invoke } from '@tauri-apps/api/tauri';
import { onMount } from 'svelte';
import { get, writable } from 'svelte/store';

import { configStore, is_tauri } from '$lib/stores';
// Import generated types
import type {
	ConfigId,
	KnowledgeGraphInputType,
	RelevanceFunction,
	ServiceType,
} from './generated/types';

const schema = writable<any>(null);

// Form types derived from generated types
type ConfigDraft = {
	id: ConfigId;
	global_shortcut: string;
	default_theme: string;
	default_role: string;
};

// HaystackForm now matches the generated Haystack type
type HaystackForm = {
	path: string;
	read_only: boolean;
	service: ServiceType;
	atomic_server_secret?: string;
	extra_parameters: { [key: string]: string };
	weight: number;
};

type KnowledgeGraphForm = {
	url: string;
	local_path: string;
	local_type: KnowledgeGraphInputType;
	public: boolean;
	publish: boolean;
};

type RoleForm = {
	name: string;
	shortname: string;
	relevance_function: RelevanceFunction;
	terraphim_it: boolean;
	theme: string;
	haystacks: HaystackForm[];
	kg: KnowledgeGraphForm;
	openrouter_enabled?: boolean;
	openrouter_api_key?: string;
	openrouter_model?: string;
	// Auto-summarize and Chat settings
	openrouter_auto_summarize?: boolean;
	openrouter_chat_enabled?: boolean;
	openrouter_chat_model?: string;
	openrouter_chat_system_prompt?: string;
	// Generic LLM abstraction settings (stored in Role.extra)
	llm_provider?: string; // openrouter | ollama
	llm_model?: string;
	llm_base_url?: string;
	llm_auto_summarize?: boolean;
};

// File selection functions for Tauri
async function _selectHaystackPath(roleIdx: number, hsIdx: number) {
	if (!get(is_tauri)) return;

	try {
		const selected = await open({
			directory: true,
			multiple: false,
		});

		if (selected && typeof selected === 'string') {
			draft.update((d) => {
				d.roles[roleIdx].haystacks[hsIdx].path = selected;
				return d;
			});
		}
	} catch (err) {
		console.error('Failed to open folder selector:', err);
	}
}

async function __selectKnowledgeGraphPath(roleIdx: number) {
	if (!get(is_tauri)) return;

	try {
		const selected = await open({
			directory: true,
			multiple: false,
		});

		if (selected && typeof selected === 'string') {
			draft.update((d) => {
				d.roles[roleIdx].kg.local_path = selected;
				return d;
			});
		}
	} catch (err) {
		console.error('Failed to open folder selector:', err);
	}
}

const draft = writable<ConfigDraft & { roles: RoleForm[] }>({
	id: 'Desktop',
	global_shortcut: 'Ctrl+X',
	default_theme: 'spacelab',
	default_role: 'Default',
	roles: [],
});

// Available Bootstrap themes
const _availableThemes = [
	'default',
	'darkly',
	'cerulean',
	'cosmo',
	'cyborg',
	'flatly',
	'journal',
	'litera',
	'lumen',
	'lux',
	'materia',
	'minty',
	'nuclear',
	'pulse',
	'sandstone',
	'simplex',
	'slate',
	'solar',
	'spacelab',
	'superhero',
	'united',
	'yeti',
];

$effect(() => {
	(async () => {
		try {
			let schemaJson: unknown;
			if (get(is_tauri)) {
				schemaJson = await invoke('get_config_schema');
			} else {
				const res = await fetch('/config/schema');
				schemaJson = await res.json();
			}
			schema.set(schemaJson);
			// initialize draft from existing config
			const current: any = get(configStore);
			if (current?.id) {
				draft.update((d) => ({
					...d,
					id: current.id,
					global_shortcut: current.global_shortcut,
					default_theme: current.roles[current.default_role]?.theme ?? 'spacelab',
					default_role: current.default_role,
					roles: Object.values(current.roles).map((r: any) => {
						const autoPath = r.kg?.automata_path;
						const url = autoPath?.Remote ?? '';
						const localPath = r.kg?.knowledge_graph_local?.path ?? '';
						return {
							name: r.name,
							shortname: r.shortname,
							relevance_function: r.relevance_function,
							terraphim_it: r.terraphim_it ?? false,
							theme: r.theme,
							haystacks: (r.haystacks ?? []).map((h: any) => ({
								path: h.location || h.path || '', // Handle both old and new field names
								read_only: h.read_only ?? false,
								service: h.service || 'Ripgrep',
								atomic_server_secret: h.atomic_server_secret || '',
								extra_parameters: h.extra_parameters || {},
								weight: h.weight ?? 1.0,
							})),
							kg: {
								url,
								local_path: localPath,
								local_type: r.kg?.knowledge_graph_local?.input_type ?? 'markdown',
								public: r.kg?.public ?? false,
								publish: r.kg?.publish ?? false,
							},
							openrouter_enabled: r.openrouter_enabled ?? false,
							openrouter_api_key: r.openrouter_api_key ?? '',
							openrouter_model: r.openrouter_model ?? 'openai/gpt-3.5-turbo',
							openrouter_auto_summarize: r.openrouter_auto_summarize ?? false,
							openrouter_chat_enabled: r.openrouter_chat_enabled ?? false,
							openrouter_chat_model:
								r.openrouter_chat_model ?? r.openrouter_model ?? 'openai/gpt-3.5-turbo',
							openrouter_chat_system_prompt: r.openrouter_chat_system_prompt ?? '',
							// Pull generic LLM settings from Role.extra if present
							llm_provider: r.extra?.llm_provider ?? '',
							llm_model: r.extra?.llm_model ?? '',
							llm_base_url: r.extra?.llm_base_url ?? '',
							llm_auto_summarize: r.extra?.llm_auto_summarize ?? false,
						};
					}),
				}));
			}
		} catch (e) {
			console.error('Failed to load schema', e);
		}
	})();
});

// Handle ESC to close wizard and return to _previous screen
$effect(() => {
	const onKeyDown = (e: KeyboardEvent) => {
		if (e.key === 'Escape') {
			if (typeof window !== 'undefined') {
				// Navigate back to main search screen
				window.history.back();
			}
		}
	};
	window.addEventListener('keydown', onKeyDown);
	return () => window.removeEventListener('keydown', onKeyDown);
});

// Cache of fetched LLM models per role index (UI-only)
let roleModels: Record<number, string[]> = {};
async function _fetchLlmModels(roleIdx: number) {
	const d = get(draft);
	const role = d.roles[roleIdx];
	const provider = role.llm_provider || (role.openrouter_enabled ? 'openrouter' : '');
	const models: string[] = [];
	try {
		if (provider === 'ollama') {
			const base = (role.llm_base_url || 'http://127.0.0.1:11434').replace(/\/$/, '');
			const res = await fetch(`${base}/api/tags`);
			const json = await res.json();
			if (Array.isArray(json?.models)) {
				for (const m of json.models) if (m?.name) models.push(m.name);
			}
		} else if (provider === 'openrouter') {
			const apiKey = role.openrouter_api_key?.trim();
			if (!apiKey) throw new Error('OpenRouter API key required');
			const res = await fetch('https://openrouter.ai/api/v1/models', {
				headers: {
					Authorization: `Bearer ${apiKey}`,
					'HTTP-Referer': 'https://terraphim.ai',
					'X-Title': 'Terraphim Desktop',
				},
			});
			const json = await res.json();
			const data = Array.isArray(json?.data) ? json.data : [];
			for (const m of data) if (m?.id) models.push(m.id);
		}
	} catch (e) {
		console.error('Failed to fetch models', e);
	}
	roleModels = { ...roleModels, [roleIdx]: models };
}

let currentStep = 1;
const totalSteps = 3;
let __saveStatus = ''; // 'success' or 'error'
function __next() {
	if (currentStep < totalSteps) currentStep += 1;
}
function __prev() {
	if (currentStep > 1) currentStep -= 1;
}

function __addRole() {
	draft.update((d) => ({
		...d,
		roles: [
			...d.roles,
			{
				name: 'New Role',
				shortname: 'new',
				relevance_function: 'title-scorer',
				terraphim_it: false,
				theme: 'spacelab',
				haystacks: [],
				kg: { url: '', local_path: '', local_type: 'markdown', public: false, publish: false },
				openrouter_enabled: false,
				openrouter_api_key: '',
				openrouter_model: 'openai/gpt-3.5-turbo',
				openrouter_auto_summarize: false,
				openrouter_chat_enabled: false,
				openrouter_chat_model: 'openai/gpt-3.5-turbo',
				openrouter_chat_system_prompt: '',
				llm_provider: '',
				llm_model: '',
				llm_base_url: '',
				llm_auto_summarize: false,
			},
		],
	}));
}
function __removeRole(idx: number) {
	draft.update((d) => ({ ...d, roles: d.roles.filter((_, i) => i !== idx) }));
}

function _addHaystack(roleIdx: number) {
	draft.update((d) => {
		d.roles[roleIdx].haystacks.push({
			path: '',
			read_only: false,
			service: 'Ripgrep',
			atomic_server_secret: '',
			extra_parameters: {},
			weight: 1.0,
		});
		return d;
	});
}
function _removeHaystack(roleIdx: number, hsIdx: number) {
	draft.update((d) => {
		d.roles[roleIdx].haystacks = d.roles[roleIdx].haystacks.filter((_, i) => i !== hsIdx);
		return d;
	});
}

// Add/remove extra parameter functions
function _addExtraParameter(roleIdx: number, hsIdx: number, key: string = '', value: string = '') {
	draft.update((d) => {
		if (!d.roles[roleIdx].haystacks[hsIdx].extra_parameters) {
			d.roles[roleIdx].haystacks[hsIdx].extra_parameters = {};
		}
		const newKey = key || `param_${Date.now()}`;
		d.roles[roleIdx].haystacks[hsIdx].extra_parameters[newKey] = value;
		return d;
	});
}

function _removeExtraParameter(roleIdx: number, hsIdx: number, paramKey: string) {
	draft.update((d) => {
		delete d.roles[roleIdx].haystacks[hsIdx].extra_parameters[paramKey];
		return d;
	});
}

function updateExtraParameterKey(roleIdx: number, hsIdx: number, oldKey: string, newKey: string) {
	draft.update((d) => {
		const params = d.roles[roleIdx].haystacks[hsIdx].extra_parameters;
		if (params[oldKey] !== undefined && oldKey !== newKey) {
			params[newKey] = params[oldKey];
			delete params[oldKey];
		}
		return d;
	});
}

function _handleParameterKeyChange(roleIdx: number, hsIdx: number, oldKey: string, event: any) {
	const newKey = event.target.value;
	updateExtraParameterKey(roleIdx, hsIdx, oldKey, newKey);
}

async function __save() {
	const data = get(draft);
	const existing = get(configStore) as any;
	const updated = { ...existing } as any;
	updated.id = data.id;
	updated.global_shortcut = data.global_shortcut;
	updated.default_role = data.default_role;

	// rebuild roles map
	const rolesMap: Record<string, any> = {};
	data.roles.forEach((r) => {
		const key = r.name;
		// clean key by removing potential wrapping quotes
		const cleanKey = key.replace(/^"|"$/g, '');
		rolesMap[cleanKey] = {
			// Preserve `extra` from existing config, or default for new roles.
			extra: existing.roles?.[key]?.extra ?? {},

			// Overwrite all other fields from the form
			name: r.name,
			shortname: r.shortname,
			theme: r.theme,
			relevance_function: r.relevance_function,
			terraphim_it: r.terraphim_it ?? false,
			haystacks: r.haystacks.map((h) => ({
				location: h.path, // Use location field as expected by backend
				service: h.service,
				read_only: h.read_only,
				atomic_server_secret: h.service === 'Atomic' ? h.atomic_server_secret : undefined,
				extra_parameters: h.extra_parameters || {},
				weight: h.weight ?? 1.0,
			})),
			kg:
				r.kg.url || r.kg.local_path
					? {
							automata_path: r.kg.url ? { Remote: r.kg.url } : null,
							knowledge_graph_local: r.kg.local_path
								? { input_type: r.kg.local_type, path: r.kg.local_path }
								: null,
							public: r.kg.public,
							publish: r.kg.publish,
						}
					: null,
			// Include OpenRouter fields if enabled
			...(r.openrouter_enabled && {
				openrouter_enabled: r.openrouter_enabled,
				openrouter_api_key: r.openrouter_api_key,
				openrouter_model: r.openrouter_model,
				openrouter_auto_summarize: r.openrouter_auto_summarize ?? false,
				openrouter_chat_enabled: r.openrouter_chat_enabled ?? false,
				openrouter_chat_model: r.openrouter_chat_model ?? r.openrouter_model,
				openrouter_chat_system_prompt: r.openrouter_chat_system_prompt ?? '',
			}),
		};

		// Merge generic LLM settings into Role.extra
		const extraUpdates: Record<string, any> = {};
		if (r.llm_provider) extraUpdates.llm_provider = r.llm_provider;
		if (r.llm_model) extraUpdates.llm_model = r.llm_model;
		if (r.llm_base_url) extraUpdates.llm_base_url = r.llm_base_url;
		if (typeof r.llm_auto_summarize === 'boolean')
			extraUpdates.llm_auto_summarize = r.llm_auto_summarize;
		rolesMap[cleanKey].extra = { ...(rolesMap[cleanKey].extra || {}), ...extraUpdates };
	});
	updated.roles = rolesMap;

	// ensure default role exists
	if (!updated.default_role || !rolesMap[updated.default_role]) {
		updated.default_role = data.roles[0]?.name ?? 'Default';
	}
	updated.selected_role = updated.default_role;

	try {
		if (get(is_tauri)) {
			await invoke('update_config', { configNew: updated });
		} else {
			const response = await fetch('/config', {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify(updated),
			});
			if (!response.ok) {
				throw new Error(`HTTP ${response.status}: ${response.statusText}`);
			}
		}
		configStore.set(updated);
		__saveStatus = 'success';
		setTimeout(() => {
			__saveStatus = '';
		}, 3000); // Clear status after 3 seconds
	} catch (e) {
		console.error(e);
		__saveStatus = 'error';
		setTimeout(() => {
			__saveStatus = '';
		}, 3000); // Clear status after 3 seconds
	}
}

function _closeWizard() {
	if (typeof window !== 'undefined') {
		window.history.back();
	}
}
</script>

<div class="box" data-testid="config-wizard">
  <div class="is-flex is-justify-content-space-between is-align-items-center" style="gap: .5rem;">
    <h3 class="title is-4" style="margin-bottom: 0;">Configuration Wizard</h3>
    <button class="button is-small is-light" on:click={_closeWizard} aria-label="Close configuration wizard">Close</button>
  </div>

  {#if __saveStatus === 'success'}
    <div class="notification is-success" data-testid="wizard-success">
      <button class="delete" on:click={() => __saveStatus = ''}></button>
      Configuration _saved successfully!
    </div>
  {/if}

  {#if __saveStatus === 'error'}
    <div class="notification is-danger" data-testid="wizard-error">
      <button class="delete" on:click={() => __saveStatus = ''}></button>
      Failed to _save configuration. Please try again.
    </div>
  {/if}
  {#if currentStep === 1}
    <!-- Step 1: Global settings -->
    <div class="field">
      <label class="label" for="config-id">Configuration ID</label>
      <div class="control">
        <div class="select">
          <select id="config-id" bind:value={$draft.id}>
            <option>Desktop</option>
            <option>Server</option>
            <option>Embedded</option>
          </select>
        </div>
      </div>
    </div>

    <div class="field">
      <label class="label" for="global-shortcut">Global shortcut</label>
      <div class="control">
        <input class="input" id="global-shortcut" type="text" bind:value={$draft.global_shortcut} />
      </div>
    </div>

    <div class="field">
      <label class="label" for="default-theme">Default theme</label>
      <div class="control">
        <div class="select is-fullwidth">
          <select id="default-theme" bind:value={$draft.default_theme}>
            {#each _availableThemes as theme}
              <option value={theme}>{theme}</option>
            {/each}
          </select>
        </div>
      </div>
    </div>

    <div class="field">
      <label class="label" for="default-role">Default Role</label>
      <div class="control">
        <div class="select">
          <select id="default-role" bind:value={$draft.default_role}>
            {#each $draft.roles as role}
              <option value={role.name}>{role.name}</option>
            {/each}
          </select>
        </div>
      </div>
    </div>
  {:else if currentStep === 2}
    <h4 class="title is-5">Roles</h4>
    {#each $draft.roles as roleItem, idx (roleItem.name)}
      <div class="box">
        <div class="field">
          <label class="label" for={`role-name-${idx}`}>Role name</label>
          <div class="control">
            <input class="input" id={`role-name-${idx}`} type="text" bind:value={$draft.roles[idx].name} />
          </div>
        </div>
        <div class="field">
          <label class="label" for={`role-shortname-${idx}`}>Short name</label>
          <div class="control">
            <input class="input" id={`role-shortname-${idx}`} type="text" bind:value={$draft.roles[idx].shortname} />
          </div>
        </div>
        <div class="field">
          <label class="label" for={`role-theme-${idx}`}>Theme</label>
          <div class="control">
            <div class="select is-fullwidth">
              <select id={`role-theme-${idx}`} bind:value={$draft.roles[idx].theme}>
                {#each _availableThemes as theme}
                  <option value={theme}>{theme}</option>
                {/each}
              </select>
            </div>
          </div>
        </div>
        <div class="field">
          <label class="label" for={`role-relevance-${idx}`}>Relevance function</label>
          <div class="control">
            <div class="select is-fullwidth">
              <select id={`role-relevance-${idx}`} bind:value={$draft.roles[idx].relevance_function}>
                <option value="title-scorer">title-scorer</option>
                <option value="terraphim-graph">terraphim-graph</option>
                <option value="bm25">bm25</option>
                <option value="bm25f">bm25f</option>
                <option value="bm25plus">bm25plus</option>
              </select>
            </div>
          </div>
        </div>

        <div class="field">
          <label class="label" for={`role-terraphim-it-${idx}`}>
            <input
              class="checkbox"
              id={`role-terraphim-it-${idx}`}
              type="checkbox"
              bind:checked={$draft.roles[idx].terraphim_it}
            />
            Enable Terraphim IT features (KG preprocessing, auto-linking)
          </label>
        </div>

        <h5 class="title is-6">Haystacks</h5>
        {#each roleItem.haystacks as hs, hIdx}
          <div class="box is-light">
            <!-- Service Type Selection -->
            <div class="field">
              <label class="label" for={`haystack-service-${idx}-${hIdx}`}>Service Type</label>
              <div class="control">
                <div class="select">
                  <select id={`haystack-service-${idx}-${hIdx}`} bind:value={$draft.roles[idx].haystacks[hIdx].service}>
                    <option value="Ripgrep">Ripgrep (File Search)</option>
                    <option value="Atomic">Atomic Server</option>
                  </select>
                </div>
              </div>
            </div>

            <!-- Path Field -->
            <div class="field">
              <label class="label" for={`haystack-path-${idx}-${hIdx}`}>
                {#if $draft.roles[idx].haystacks[hIdx].service === "Atomic"}
                  Server URL
                {:else}
                  Directory Path
                {/if}
              </label>
              <div class="control">
                <input
                  class="input"
                  id={`haystack-path-${idx}-${hIdx}`}
                  type="text"
                  placeholder={$draft.roles[idx].haystacks[hIdx].service === "Atomic" ? "https://localhost:9883" : "/path/to/documents"}
                  bind:value={$draft.roles[idx].haystacks[hIdx].path}
                  readonly={$is_tauri && $draft.roles[idx].haystacks[hIdx].service !== "Atomic"}
                  on:click={$is_tauri && $draft.roles[idx].haystacks[hIdx].service !== "Atomic" ? () => _selectHaystackPath(idx, hIdx) : undefined}
                />
                {#if $is_tauri && $draft.roles[idx].haystacks[hIdx].service !== "Atomic"}
                  <p class="help">Click to select directory</p>
                {/if}
              </div>
            </div>

            <!-- Atomic Server Secret (only for Atomic service) -->
            {#if $draft.roles[idx].haystacks[hIdx].service === "Atomic"}
              <div class="field">
                <label class="label" for={`haystack-secret-${idx}-${hIdx}`}>Atomic Server Secret</label>
                <div class="control">
                  <input
                    class="input"
                    id={`haystack-secret-${idx}-${hIdx}`}
                    type="password"
                    placeholder="Base64 encoded secret (optional)"
                    bind:value={$draft.roles[idx].haystacks[hIdx].atomic_server_secret}
                  />
                </div>
                <p class="help">Leave empty for anonymous access</p>
              </div>
            {/if}

            <!-- Extra Parameters (only for Ripgrep service) -->
            {#if $draft.roles[idx].haystacks[hIdx].service === "Ripgrep"}
              <div class="field">
                <label class="label">Extra Parameters (for filtering)</label>
                <!-- Dedicated Hashtag field mapped to extra_parameters.tag -->
                <div class="field is-grouped">
                  <div class="control">
                    <label class="label" for={`ripgrep-hashtag-${idx}-${hIdx}`}>Hashtag</label>
                    <input
                      class="input"
                      id={`ripgrep-hashtag-${idx}-${hIdx}`}
                      type="text"
                      placeholder="#rust"
                      bind:value={$draft.roles[idx].haystacks[hIdx].extra_parameters["tag"]}
                    />
                  </div>
                </div>
                <div class="field is-grouped" style="margin-bottom:.5rem;">
                  <div class="control">
                    <label class="label" for={`ripgrep-hashtag-preset-${idx}-${hIdx}`}>Presets</label>
                    <div class="select is-small">
                      <select id={`ripgrep-hashtag-preset-${idx}-${hIdx}`}
                        on:change={(e)=>{
                          const val = e.currentTarget.value;
                          if (val) { $draft.roles[idx].haystacks[hIdx].extra_parameters["tag"] = val; }
                        }}>
                        <option value="">(choose)</option>
                        <option value="#rust">#rust</option>
                        <option value="#docs">#docs</option>
                        <option value="#test">#test</option>
                        <option value="#todo">#todo</option>
                      </select>
                    </div>
                  </div>
                </div>
                <p class="help">When set, searches will enforce the hashtag alongside your query (AND), e.g. <code>-e "search" -e "#rust"</code>.</p>
                {#each Object.entries($draft.roles[idx].haystacks[hIdx].extra_parameters || {}) as [paramKey, paramValue], paramIdx}
                  <div class="field is-grouped">
                    <div class="control">
                      <input
                        class="input"
                        type="text"
                        placeholder="Parameter name"
                        value={paramKey}
                        on:blur={(e) => _handleParameterKeyChange(idx, hIdx, paramKey, e)}
                      />
                    </div>
                    <div class="control is-expanded">
                      <input
                        class="input"
                        type="text"
                        placeholder="Parameter value"
                        bind:value={$draft.roles[idx].haystacks[hIdx].extra_parameters[paramKey]}
                      />
                    </div>
                    <div class="control">
                      <button
                        class="button is-small is-danger"
                        on:click={() => _removeExtraParameter(idx, hIdx, paramKey)}
                      >
                        ×
                      </button>
                    </div>
                  </div>
                {/each}

                <!-- Predefined parameter buttons for common use cases -->
                <div class="field is-grouped">
                  <div class="control">
                    <button
                      class="button is-small is-link is-light"
                      on:click={() => _addExtraParameter(idx, hIdx, "tag", "#rust")}
                    >
                      + Tag Filter
                    </button>
                  </div>
                  <div class="control">
                    <button
                      class="button is-small is-link is-light"
                      on:click={() => _addExtraParameter(idx, hIdx, "max_count", "10")}
                    >
                      + Max Results
                    </button>
                  </div>
                  <div class="control">
                    <button
                      class="button is-small is-link is-light"
                      on:click={() => _addExtraParameter(idx, hIdx, "", "")}
                    >
                      + Custom Parameter
                    </button>
                  </div>
                </div>

                <p class="help">
                  Common parameters: <code>tag</code> (e.g., "#rust"), <code>glob</code> (e.g., "*.md"),
                  <code>max_count</code> (e.g., "10"), <code>context</code> (e.g., "5")
                </p>
              </div>
            {/if}

            <!-- Read-only checkbox -->
            <div class="field">
              <label class="checkbox" for={`haystack-readonly-${idx}-${hIdx}`}>
                <input id={`haystack-readonly-${idx}-${hIdx}`} type="checkbox" bind:checked={$draft.roles[idx].haystacks[hIdx].read_only} />
                &nbsp;Read-only
              </label>
            </div>

            <!-- Weight field -->
            <div class="field">
              <label class="label" for={`haystack-weight-${idx}-${hIdx}`}>Ranking Weight</label>
              <div class="control">
                <input
                  class="input"
                  id={`haystack-weight-${idx}-${hIdx}`}
                  type="number"
                  min="0"
                  max="10"
                  step="0.1"
                  placeholder="1.0"
                  bind:value={$draft.roles[idx].haystacks[hIdx].weight}
                />
              </div>
              <p class="help">Weight for ranking results (1.0 = normal, &gt;1.0 = higher priority, &lt;1.0 = lower priority)</p>
            </div>

            <!-- Remove haystack button -->
            <div class="field">
              <button class="button is-small is-danger" data-testid="remove-haystack-{idx}-{hIdx}" on:click={() => _removeHaystack(idx, hIdx)}>
                Remove Haystack
              </button>
            </div>
          </div>
        {/each}
        <button class="button is-small" data-testid="add-haystack-{idx}" on:click={() => _addHaystack(idx)}>Add Haystack</button>

        <!-- LLM Provider (Generic) -->
        <h5 class="title is-6">AI-Enhanced Summaries (LLM Provider)</h5>
        <div class="field">
          <label class="label" for={`llm-provider-${idx}`}>Provider</label>
          <div class="control">
            <div class="select">
              <select id={`llm-provider-${idx}`} bind:value={$draft.roles[idx].llm_provider}>
                <option value="">(none)</option>
                <option value="openrouter">OpenRouter</option>
                <option value="ollama">Ollama (local)</option>
              </select>
            </div>
          </div>
          <p class="help">Choose a provider. OpenRouter uses API key; Ollama runs locally.</p>
        </div>

        <!-- Generic LLM fields -->
        {#if $draft.roles[idx].llm_provider === 'ollama'}
          <div class="field">
            <label class="label" for={`llm-model-${idx}`}>Model</label>
            <div class="control">
              <input class="input" id={`llm-model-${idx}`} type="text" placeholder="llama3.1" bind:value={$draft.roles[idx].llm_model} />
            </div>
            <button class="button is-small" on:click={() => _fetchLlmModels(idx)}>Fetch models</button>
            {#if roleModels[idx]?.length}
              <div class="select is-fullwidth" style="margin-top:0.5rem;">
                                        <select on:change={(e)=>{$draft.roles[idx].llm_model=e.currentTarget.value}}>
                  {#each roleModels[idx] as m}
                    <option value={m}>{m}</option>
                  {/each}
                </select>
              </div>
            {/if}
          </div>
          <div class="field">
            <label class="label" for={`llm-base-url-${idx}`}>Base URL</label>
            <div class="control">
              <input class="input" id={`llm-base-url-${idx}`} type="text" placeholder="http://127.0.0.1:11434" bind:value={$draft.roles[idx].llm_base_url} />
            </div>
          </div>
          <div class="field">
            <label class="checkbox" for={`llm-auto-summarize-${idx}`}>
              <input id={`llm-auto-summarize-${idx}`} type="checkbox" bind:checked={$draft.roles[idx].llm_auto_summarize} />
              &nbsp;Automatically summarize search results
            </label>
          </div>
        {/if}

        <!-- OpenRouter AI Configuration -->
        <h5 class="title is-6">AI-Enhanced Summaries (OpenRouter)</h5>
        <div class="field">
          <label class="checkbox" for={`openrouter-enabled-${idx}`}>
            <input id={`openrouter-enabled-${idx}`} type="checkbox" bind:checked={$draft.roles[idx].openrouter_enabled} />
            &nbsp;Enable AI-generated article summaries
          </label>
          <p class="help">Generate intelligent summaries using OpenRouter's language models</p>
        </div>

        {#if $draft.roles[idx].openrouter_enabled}
          <div class="field">
            <label class="label" for={`openrouter-api-key-${idx}`}>OpenRouter API Key</label>
            <div class="control">
              <input
                class="input"
                id={`openrouter-api-key-${idx}`}
                type="password"
                placeholder="sk-or-v1-..."
                bind:value={$draft.roles[idx].openrouter_api_key}
              />
            </div>
            <p class="help">Get your API key from <a href="https://openrouter.ai" target="_blank" rel="noopener">OpenRouter</a></p>
          </div>

          <div class="field">
            <label class="label" for={`openrouter-model-${idx}`}>Model</label>
            <div class="control">
              <input class="input" id={`openrouter-model-${idx}`} type="text" placeholder="openai/gpt-4-turbo" bind:value={$draft.roles[idx].openrouter_model} />
            </div>
            <button class="button is-small" on:click={() => _fetchLlmModels(idx)}>Fetch models</button>
            {#if roleModels[idx]?.length}
              <div class="select is-fullwidth" style="margin-top:0.5rem;">
                                        <select on:change={(e)=>{$draft.roles[idx].openrouter_model=e.currentTarget.value}}>
                  {#each roleModels[idx] as m}
                    <option value={m}>{m}</option>
                  {/each}
                </select>
              </div>
            {/if}
            <p class="help">Choose the language model for generating summaries. Different models offer different speed/quality tradeoffs.</p>
          </div>

          <div class="field">
            <label class="checkbox" for={`openrouter-auto-summarize-${idx}`}>
              <input id={`openrouter-auto-summarize-${idx}`} type="checkbox" bind:checked={$draft.roles[idx].openrouter_auto_summarize} />
              &nbsp;Automatically summarize search results
            </label>
            <p class="help">When enabled, summaries will be generated and shown in search results.</p>
          </div>

          <div class="field">
            <label class="checkbox" for={`openrouter-chat-enabled-${idx}`}>
              <input id={`openrouter-chat-enabled-${idx}`} type="checkbox" bind:checked={$draft.roles[idx].openrouter_chat_enabled} />
              &nbsp;Enable Chat interface (OpenRouter)
            </label>
          </div>

          {#if $draft.roles[idx].openrouter_chat_enabled}
            <div class="field">
              <label class="label" for={`openrouter-chat-model-${idx}`}>Chat Model</label>
              <div class="control">
                <div class="select is-fullwidth">
                  <select id={`openrouter-chat-model-${idx}`} bind:value={$draft.roles[idx].openrouter_chat_model}>
                    <option value="openai/gpt-3.5-turbo">GPT-3.5 Turbo</option>
                    <option value="openai/gpt-4">GPT-4</option>
                    <option value="anthropic/claude-3-sonnet">Claude 3 Sonnet</option>
                    <option value="anthropic/claude-3-haiku">Claude 3 Haiku</option>
                    <option value="mistralai/mixtral-8x7b-instruct">Mixtral 8x7B</option>
                  </select>
                </div>
              </div>
            </div>

            <div class="field">
              <label class="label" for={`openrouter-chat-system-${idx}`}>System Prompt (optional)</label>
              <div class="control">
                <textarea class="textarea" id={`openrouter-chat-system-${idx}`} rows="3" placeholder="You are a helpful Rust engineer assistant..." bind:value={$draft.roles[idx].openrouter_chat_system_prompt}></textarea>
              </div>
            </div>
          {/if}
        {/if}

        <h5 class="title is-6">Knowledge Graph</h5>
        <div class="field">
          <label class="label" for={`kg-url-${idx}`}>Remote automata URL</label>
          <div class="control">
            <input class="input" id={`kg-url-${idx}`} type="text" placeholder="https://example.com/thesaurus.json" bind:value={$draft.roles[idx].kg.url} />
          </div>
        </div>
        <div class="field">
          <label class="label" for={`kg-local-path-${idx}`}>Local KG path</label>
          <div class="control">
            <input
              class="input"
              id={`kg-local-path-${idx}`}
              type="text"
              placeholder="/path/to/markdown"
              bind:value={$draft.roles[idx].kg.local_path}
              readonly={$is_tauri}
              on:click={$is_tauri ? () => __selectKnowledgeGraphPath(idx) : undefined}
            />
            {#if $is_tauri}
              <p class="help">Click to select directory</p>
            {/if}
          </div>
        </div>
        <div class="field is-grouped">
          <div class="control">
            <label class="label" for={`kg-local-type-${idx}`}>Local KG type</label>
            <div class="select">
              <select id={`kg-local-type-${idx}`} bind:value={$draft.roles[idx].kg.local_type}>
                <option value="markdown">markdown</option>
                <option value="json">json</option>
              </select>
            </div>
          </div>
        </div>
        <div class="field is-grouped">
          <label class="checkbox" for={`kg-public-${idx}`} style="margin-right:1rem">
            <input id={`kg-public-${idx}`} type="checkbox" bind:checked={$draft.roles[idx].kg.public} />
            &nbsp;Public
          </label>
          <label class="checkbox" for={`kg-publish-${idx}`}>
            <input id={`kg-publish-${idx}`} type="checkbox" bind:checked={$draft.roles[idx].kg.publish} />
            &nbsp;Publish
          </label>
        </div>
        <hr />
        <button class="button is-small is-danger" data-testid="remove-role-{idx}" on:click={() => __removeRole(idx)}>Remove Role</button>
      </div>
    {/each}
    <button class="button is-link is-light" data-testid="add-role" on:click={__addRole}>Add Role</button>
  {:else}
    <h4 class="title is-5">Review</h4>
    <div class="content">
      <h5>Configuration Summary:</h5>
      <ul>
        <li><strong>Configuration ID:</strong> {$draft.id}</li>
        <li><strong>Global Shortcut:</strong> {$draft.global_shortcut}</li>
        <li><strong>Default Theme:</strong> {$draft.default_theme}</li>
        <li><strong>Default Role:</strong> {$draft.default_role}</li>
      </ul>

      <h5>Roles:</h5>
      {#each $draft.roles as role, idx}
        <div class="box">
          <h6 data-testid="review-role-name-{idx}">{role.name}</h6>
          <p><strong>Shortname:</strong> {role.shortname}</p>
          <p><strong>Theme:</strong> {role.theme}</p>
          <p><strong>Relevance Function:</strong> {role.relevance_function}</p>
          <button class="button is-small" data-testid="edit-role-{idx}" on:click={() => { currentStep = 2; }}>Edit Role</button>
        </div>
      {/each}
    </div>
    <pre style="max-height:300px;overflow:auto">{JSON.stringify($draft, null, 2)}</pre>
  {/if}

  <nav class="level">
    <div class="level-left">
      {#if currentStep > 1}
        <button class="button" data-testid="wizard-back" on:click={__prev}>Back</button>
      {/if}
    </div>
    <div class="level-right">
      {#if currentStep < totalSteps}
        <button class="button is-primary" data-testid="wizard-_next" on:click={__next}>Next</button>
      {:else}
        <button class="button is-success" data-testid="wizard-_save" on:click={__save}>Save</button>
      {/if}
    </div>
  </nav>
</div>
