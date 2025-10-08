# TruthForge Deployment Progress

## Current Status: BLOCKED - Port Configuration Issue

### Completed Tasks
✅ Created TruthForge-specific configuration with 7 specialized crisis analysis roles:
  - BiasDetector
  - NarrativeMapper  
  - OmissionDetector
  - TaxonomyLinker
  - SupportingAgent (Exploitation)
  - OpposingAgent (Defense)
  - CrisisAnalyst (default)

✅ Fixed truthforge_config.json `id` field from "TruthForge" to "Server"

✅ Created settings.toml with port 8091 configuration

✅ Made UI responsive (removed fixed textarea rows, added min-height CSS)

✅ Integrated Terraphim Settings Modal (settings-modal.html + settings-ui.js)

✅ Rewrote app.js to use proper Terraphim API:
  - TerraphimApiClient instead of custom endpoint
  - `/chat` completion API with role parameter
  - `/config` endpoint for role loading
  - Changed default URL from localhost to https://truthforge-api.terraphim.cloud

### Current Blocker

**Problem**: TruthForge server ignores port configuration in settings.toml

**Evidence**:
- settings.toml exists at `/home/alex/infrastructure/terraphim-private-cloud-new/settings.toml` with `server_hostname = "127.0.0.1:8091"`
- Server still starts on port 8000 (log shows "listening on http://127.0.0.1:8000")
- Start script runs from `/home/alex/infrastructure/terraphim-private-cloud-new` directory

**Investigation**:
- Port configuration comes from `DeviceSettings::load_from_env_and_file(None)` in terraphim_server/src/main.rs:58
- Settings loading looks for settings.toml in multiple locations
- The server may be loading a different settings.toml file (default from crates/terraphim_settings/default/settings.toml)

### Next Steps Required

1. **Determine settings loading order**: Find out which settings.toml file the server actually loads
2. **Fix settings path**: Either:
   - Use `TERRAPHIM_SETTINGS_PATH` environment variable to override
   - Place settings.toml in the correct location
   - Pass settings path as CLI argument (if supported)

3. **Restart server with correct port** (8091)

4. **Configure Caddy reverse proxy** for public access at https://truthforge-api.terraphim.cloud

5. **Test end-to-end workflow**:
   - Verify `/config` returns TruthForge roles
   - Test crisis narrative analysis through full two-pass workflow
   - Validate all 7 agents respond correctly

### Architecture Overview

**Main Server**: Port 8090 (running, confirmed)
**TruthForge Server**: Should be port 8091 (currently incorrectly on 8000)

**UI**: https://truthforge.terraphim.cloud → TruthForge-UI
**API**: https://truthforge-api.terraphim.cloud → TruthForge Server (port 8091 via Caddy)

### Files Deployed

**Local**:
- `/home/alex/projects/terraphim/terraphim-ai/examples/agent-workflows/6-truthforge-debate/app.js` (updated)
- `/home/alex/projects/terraphim/terraphim-ai/examples/agent-workflows/6-truthforge-debate/index.html` (responsive CSS)

**Remote (bigbox)**:
- `/home/alex/infrastructure/terraphim-private-cloud-new/truthforge_config.json` (TruthForge roles)
- `/home/alex/infrastructure/terraphim-private-cloud-new/settings.toml` (port 8091 config)
- `/home/alex/infrastructure/terraphim-private-cloud-new/start-truthforge.sh` (start script)
- `/home/alex/infrastructure/terraphim-private-cloud-new/truthforge-ui/app.js` (API client)
- `/home/alex/infrastructure/terraphim-private-cloud-new/truthforge-ui/index.html` (UI)
- `/home/alex/infrastructure/terraphim-private-cloud-new/truthforge-ui/settings-modal.html` (settings UI)

### Key Technical Decisions

1. **Port configuration**: settings.toml (not environment variables or CLI args)
2. **Public API URL**: https://truthforge-api.terraphim.cloud (not localhost)
3. **Terraphim API integration**: Using official `/chat` and `/config` endpoints (not custom endpoints)
4. **Role-based analysis**: Each Pass uses different roles with specialized system prompts
