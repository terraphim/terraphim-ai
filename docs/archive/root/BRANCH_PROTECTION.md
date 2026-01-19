# Branch Protection and Naming Conventions

## Overview

This repository has comprehensive protection rules to prevent private branches from leaking into the public repository. This document outlines the protection mechanisms and naming conventions.

## Branch Protection Rules

### Main Branch Protection
- **Required Status Checks**: All CI checks must pass before merging
- **Required Reviews**: At least 1 approval required for pull requests
- **Admin Enforcement**: Even admins must follow protection rules
- **Force Push Prevention**: Force pushes are disabled
- **Branch Deletion Prevention**: Branch deletion is disabled
- **Auto-merge**: Disabled to ensure manual review

### Private Branch Detection

The repository uses a multi-layered approach to detect and prevent private branches:

#### 1. Pre-Push Hook Validation
A pre-push hook runs before any push to the public repository and blocks branches matching these patterns:

**Branch Naming Patterns (Blocked):**
- `private-*` or `private_*` (e.g., `private-feature`, `private_tf`)
- `internal-*` or `internal_*` (e.g., `internal-docs`, `internal_api`)
- `client-*` or `client_*` (e.g., `client-data`, `client_config`)
- `secret-*` or `secret_*` (e.g., `secret-key`, `secret_auth`)
- `wip-private-*` or `wip-private_*` (e.g., `wip-private-feature`)
- `customer-*` or `customer_*` (e.g., `customer-data`, `customer_api`)
- `proprietary-*` or `proprietary_*` (e.g., `proprietary-code`)
- `confidential-*` or `confidential_*` (e.g., `confidential-docs`)

**Explicit Private Branches:**
- `sqlite_haystack`
- `private-feature`
- `internal-docs`
- `customer-data`
- `private_tf`

#### 2. Commit Message Validation
Commits with these markers are blocked:
- `[PRIVATE]`, `[INTERNAL]`, `[CLIENT]`, `[DO-NOT-PUSH]`
- `[CONFIDENTIAL]`, `[CUSTOMER]`
- `private:`, `internal:`, `client:`, `secret:`, `confidential:`

#### 3. Sensitive Content Detection
File contents are scanned for sensitive keywords:
- `customer_api_key`, `client_secret`, `private_key_prod`
- `internal_endpoint`, `zestic.*api`, `customer.*password`
- `truthforge`, `private.*cloud`, `internal.*api`

#### 4. File Pattern Detection
Files matching patterns in `.gitprivateignore` are blocked (when file exists).

## Repository Structure

### Public Repository (`terraphim/terraphim-ai`)
- Contains only public, open-source code
- All branches must pass pre-push validation
- Protected by comprehensive branch protection rules

### Private Repository (`zestic-ai/terraphim-private`)
- Contains private, proprietary, and client-specific code
- No validation restrictions
- Used for internal development and sensitive features

## Branch Naming Guidelines

### ✅ Allowed Branch Names
- `feature/new-ui-component`
- `fix/memory-leak`
- `docs/update-readme`
- `refactor/cleanup-code`
- `test/add-unit-tests`

### ❌ Blocked Branch Names
- `private-feature` → Use private repository
- `internal-api` → Use private repository
- `client-data` → Use private repository
- `secret-auth` → Use private repository
- `private_tf` → Use private repository

## Workflow

### For Public Development
1. Create feature branch with descriptive name
2. Make changes and commit
3. Push to public repository (validated by pre-push hook)
4. Create pull request to main branch
5. Wait for CI checks and review approval
6. Merge after approval

### For Private Development
1. Switch to private remote: `git remote set-url origin git@github.com:zestic-ai/terraphim-private.git`
2. Create private branch: `git checkout -b private-feature`
3. Make changes and commit
4. Push to private repository: `git push origin private-feature`
5. Continue development in private repository

## Troubleshooting

### Pre-Push Hook Blocked My Branch
If you see this error:
```
✗ Branch 'private_tf' matches private pattern '^private_'
Private branches should not be pushed to public remotes.
Push to private remote instead: git push private private_tf
```

**Solution:**
1. Configure branch to push to private remote:
   ```bash
   git config branch.private_tf.remote private
   git config branch.private_tf.pushRemote private
   ```
2. Push to private repository:
   ```bash
   git push private private_tf
   ```

### Branch Protection Rules Blocked Merge
If you see branch protection errors:
1. Ensure all CI checks are passing
2. Get required number of approvals
3. Ensure you're not trying to force push
4. Check that you have proper permissions

## Security Features

### Pre-Push Hook Features
- **Multi-pattern matching**: Catches various private branch naming conventions
- **Commit message scanning**: Detects private markers in commit messages
- **Content scanning**: Scans file contents for sensitive keywords
- **File pattern matching**: Uses `.gitprivateignore` for custom patterns
- **Clear error messages**: Provides specific guidance on how to fix issues
- **Audit logging**: Logs all push attempts for security monitoring

### Repository Settings
- **Branch deletion protection**: Prevents accidental branch deletion
- **Force push prevention**: Prevents history rewriting
- **Required status checks**: Ensures code quality before merging
- **Review requirements**: Ensures code review before merging
- **Admin enforcement**: Even admins must follow rules

## Configuration Files

### Pre-Push Hook
Location: `.git/hooks/pre-push`
- Automatically runs before any push
- Configurable patterns and keywords
- Can be updated to add new detection patterns

### Git Configuration
```bash
# Configure branch to push to private remote
git config branch.<branch-name>.remote private
git config branch.<branch-name>.pushRemote private

# Check current remote configuration
git remote -v
```

## Monitoring and Alerts

The pre-push hook logs all push attempts to `.git/push-audit.log` for security monitoring. Failed attempts are logged with timestamps and reasons for blocking.

## Updates and Maintenance

To update the protection rules:
1. Modify `.git/hooks/pre-push` for hook changes
2. Update this documentation
3. Test changes with non-sensitive test branches
4. Deploy changes to all developer environments

---

**Remember**: When in doubt, use the private repository for any code that might contain sensitive information, client data, or proprietary algorithms.
