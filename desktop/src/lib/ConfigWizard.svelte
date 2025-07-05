<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/tauri";
  // @ts-ignore
  import { is_tauri } from "$lib/stores";
  import { writable, get } from "svelte/store";
  // @ts-ignore
  import { configStore } from "$lib/stores";

  const schema = writable<any>(null);
  type ConfigDraft = {
    id: string;
    global_shortcut: string;
    default_theme: string;
    default_role: string;
  };
  type HaystackForm = { path: string; read_only: boolean };
  type KnowledgeGraphForm = { url: string; local_path: string; local_type: string; public: boolean; publish: boolean };
  type RoleForm = { name: string; shortname: string; relevance_function: string; theme: string; haystacks: HaystackForm[]; kg: KnowledgeGraphForm };
  const draft = writable<ConfigDraft & { roles: RoleForm[] }>({
    id: "Desktop",
    global_shortcut: "Ctrl+X",
    default_theme: "spacelab",
    default_role: "Default",
    roles: []
  });

  onMount(async () => {
    try {
      let schemaJson;
      if (get(is_tauri)) {
        schemaJson = await invoke("get_config_schema");
      } else {
        const res = await fetch("/config/schema");
        schemaJson = await res.json();
      }
      schema.set(schemaJson);
      // initialize draft from existing config
      const current: any = get(configStore);
      if (current && current.id) {
        draft.update((d) => ({
          ...d,
          id: current.id,
          global_shortcut: current.global_shortcut,
          default_theme: current.roles[current.default_role]?.theme ?? "spacelab",
          default_role: current.default_role,
          roles: Object.values(current.roles).map((r:any)=>{
            const autoPath = r.kg?.automata_path;
            const url = autoPath?.Remote ?? "";
            const localPath = r.kg?.knowledge_graph_local?.path ?? "";
            return {
              name: r.name,
              shortname: r.shortname,
              relevance_function: r.relevance_function,
              theme: r.theme,
              haystacks: (r.haystacks ?? []).map((h:any)=>({path:h.path, read_only:h.read_only ?? false})),
              kg: { url, local_path: localPath, local_type: r.kg?.knowledge_graph_local?.input_type ?? "markdown", public: r.kg?.public ?? false, publish: r.kg?.publish ?? false }
            };
          })
        }));
      }
    } catch (e) {
      console.error("Failed to load schema", e);
    }
  });

  let currentStep = 1;
  const totalSteps = 3;
  function next() {
    if (currentStep < totalSteps) currentStep += 1;
  }
  function prev() {
    if (currentStep > 1) currentStep -= 1;
  }

  function addRole() {
    draft.update((d) => ({ ...d, roles: [...d.roles, { name: "New Role", shortname:"new", relevance_function: "TitleScorer", theme: "spacelab", haystacks: [], kg:{url:"", local_path:"", local_type:"markdown", public:false, publish:false} }] }));
  }
  function removeRole(idx: number) {
    draft.update((d) => ({ ...d, roles: d.roles.filter((_, i) => i !== idx) }));
  }

  function addHaystack(roleIdx:number){
    draft.update(d=>{
      d.roles[roleIdx].haystacks.push({path:"", read_only:false});
      return d;
    });
  }
  function removeHaystack(roleIdx:number, hsIdx:number){
    draft.update(d=>{
      d.roles[roleIdx].haystacks=d.roles[roleIdx].haystacks.filter((_,i)=>i!==hsIdx);
      return d;
    })
  }

  async function save() {
    const data = get(draft);
    const existing = get(configStore) as any;
    let updated = { ...existing } as any;
    updated.id = data.id;
    updated.global_shortcut = data.global_shortcut;
    updated.default_role = data.default_role;

    // rebuild roles map
    const rolesMap: Record<string, any> = {};
    data.roles.forEach((r) => {
      const key = r.name;
      // clean key by removing potential wrapping quotes
      const cleanKey = key.replace(/^"|"$/g, "");
      rolesMap[cleanKey] = {
        // Preserve `extra` from existing config, or default for new roles.
        extra: existing.roles?.[key]?.extra ?? {},

        // Overwrite all other fields from the form
        name: r.name,
        shortname: r.shortname,
        theme: r.theme,
        relevance_function: r.relevance_function,
        haystacks: r.haystacks.map((h)=>({path: h.path, service:"Ripgrep", read_only: h.read_only})),
        kg: r.kg.url || r.kg.local_path ? {
          automata_path: r.kg.url ? { Remote: r.kg.url } : null,
          knowledge_graph_local: r.kg.local_path ? { input_type: r.kg.local_type, path: r.kg.local_path } : null,
          public: r.kg.public,
          publish: r.kg.publish
        } : null
      };
    });
    updated.roles = rolesMap;

    // ensure default role exists
    if (!updated.default_role || !rolesMap[updated.default_role]) {
      updated.default_role = data.roles[0]?.name ?? "Default";
    }
    updated.selected_role = updated.default_role;

    try {
      if (get(is_tauri)) {
        await invoke("update_config", { configNew: updated });
      } else {
        await fetch("/config", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(updated),
        });
      }
      configStore.set(updated);
      alert("Configuration saved");
    } catch (e) {
      console.error(e);
      alert("Failed to save config");
    }
  }
</script>

<div class="box">
  <h3 class="title is-4">Configuration Wizard</h3>
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
        <input class="input" id="default-theme" type="text" bind:value={$draft.default_theme} />
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
            <input class="input" id={`role-theme-${idx}`} type="text" bind:value={$draft.roles[idx].theme} />
          </div>
        </div>
        <div class="field">
          <label class="label" for={`role-relevance-${idx}`}>Relevance function</label>
          <div class="control">
            <input class="input" id={`role-relevance-${idx}`} type="text" bind:value={$draft.roles[idx].relevance_function} />
          </div>
        </div>
        <h5 class="title is-6">Haystacks</h5>
        {#each roleItem.haystacks as hs, hIdx}
          <div class="field is-grouped">
            <div class="control is-expanded">
              <label class="label" for={`haystack-path-${idx}-${hIdx}`}>Path</label>
              <input class="input" id={`haystack-path-${idx}-${hIdx}`} type="text" placeholder="/path/to/haystack" bind:value={$draft.roles[idx].haystacks[hIdx].path} />
            </div>
            <label class="checkbox" for={`haystack-readonly-${idx}-${hIdx}`} style="margin-right:0.5rem">
              <input id={`haystack-readonly-${idx}-${hIdx}`} type="checkbox" bind:checked={$draft.roles[idx].haystacks[hIdx].read_only} />
              &nbsp;Read-only
            </label>
            <button class="button is-small is-danger" on:click={() => removeHaystack(idx,hIdx)}>-</button>
          </div>
        {/each}
        <button class="button is-small" on:click={() => addHaystack(idx)}>Add Haystack</button>

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
            <input class="input" id={`kg-local-path-${idx}`} type="text" placeholder="/path/to/markdown" bind:value={$draft.roles[idx].kg.local_path} />
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
        <button class="button is-small is-danger" on:click={() => removeRole(idx)}>Remove Role</button>
      </div>
    {/each}
    <button class="button is-link is-light" on:click={addRole}>Add Role</button>
  {:else}
    <h4 class="title is-5">Review</h4>
    <pre style="max-height:300px;overflow:auto">{JSON.stringify($draft, null, 2)}</pre>
  {/if}

  <nav class="level">
    <div class="level-left">
      {#if currentStep > 1}
        <button class="button" on:click={prev}>Back</button>
      {/if}
    </div>
    <div class="level-right">
      {#if currentStep < totalSteps}
        <button class="button is-primary" on:click={next}>Next</button>
      {:else}
        <button class="button is-success" on:click={save}>Save</button>
      {/if}
    </div>
  </nav>
</div> 