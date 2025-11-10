# GitHub Actions + 1Password Integration (Aligned with PR #290)

This guide documents how our CI uses 1Password to fetch secrets directly from the vault at build time, without storing long‑lived secrets in GitHub. The only GitHub secret required is `OP_SERVICE_ACCOUNT_TOKEN`.

## Prerequisites

- Create a 1Password Service Account scoped to the vault containing release keys (e.g., `TerraphimPlatform`).
- Add the service account token to the repository: `Settings → Secrets and variables → Actions → New repository secret` named `OP_SERVICE_ACCOUNT_TOKEN`.
- Ensure these items exist in the vault and are readable by the service account:
  - `tauri.update.signing / TAURI_PRIVATE_KEY` (concealed)
  - `tauri.update.signing / TAURI_PUBLIC_KEY` (text)

## Recommended Pattern (load-secrets-action@v2)

```yaml
- name: Install 1Password CLI
  uses: 1password/install-cli-action@v1

- name: Load 1Password secrets
  uses: 1password/load-secrets-action@v2
  with:
    export-env: true
  env:
    OP_SERVICE_ACCOUNT_TOKEN: ${{ secrets.OP_SERVICE_ACCOUNT_TOKEN }}
  secrets: |
    TAURI_PRIVATE_KEY=op://TerraphimPlatform/tauri.update.signing/TAURI_PRIVATE_KEY
    TAURI_PUBLIC_KEY=op://TerraphimPlatform/tauri.update.signing/TAURI_PUBLIC_KEY

# If your config template includes op:// references, inject them using the CLI
- name: Generate Tauri config from template
  working-directory: ./desktop
  env:
    OP_SERVICE_ACCOUNT_TOKEN: ${{ secrets.OP_SERVICE_ACCOUNT_TOKEN }}
  run: |
    op inject --force -i src-tauri/tauri.conf.json.template -o src-tauri/tauri.conf.json

# Build with signing key provided from 1Password via environment
- name: Build Tauri app (signed)
  working-directory: ./desktop
  env:
    TAURI_PRIVATE_KEY: ${{ env.TAURI_PRIVATE_KEY }}
  run: yarn tauri build
```

Notes:
- Use `load-secrets-action@v2` to export environment variables from 1Password directly.
- Use `op inject` only when you must render templates containing `op://` references (e.g., public keys in `tauri.conf.json`).

## Where This Is Used

- `.github/workflows/publish-tauri.yml` — publishes signed Tauri builds and `latest.json` (auto‑update).
- `.github/workflows/release-comprehensive.yml` — includes desktop build matrix; now loads secrets for signing.

## Troubleshooting

- Missing or invalid `OP_SERVICE_ACCOUNT_TOKEN` → 1Password steps fail. Verify the secret exists and the service account has read access to the vault.
- `op inject` cannot resolve references → verify vault/item/field names match the `op://Vault/Item/Field` scheme.
- Unsigned builds → ensure `TAURI_PRIVATE_KEY` is present in the loaded secrets; Tauri will skip signing if not provided.

## Security

- Only the service account token is stored in GitHub secrets; actual signing keys live in 1Password.
- Restrict the service account to read‑only access on the minimal vault scope.

This reflects the approach introduced in PR #290 and is now the canonical method for CI secret handling.
