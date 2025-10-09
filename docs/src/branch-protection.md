# Branch Protection and Security

## Overview

Terraphim AI implements comprehensive branch protection to prevent private or sensitive code from leaking into the public repository. This system uses multiple layers of protection including pre-push hooks, branch naming conventions, and repository-level security rules.

## Repository Architecture

### Dual-Repository Approach

Terraphim AI uses a **dual-repository architecture** for security:

- **Public Repository** (`terraphim/terraphim-ai`): Open-source code only
- **Private Repository** (`zestic-ai/terraphim-private`): Proprietary and sensitive code

This separation ensures that:
- Open-source development remains transparent and accessible
- Proprietary algorithms and client data stay secure
- Compliance with various licensing and security requirements

## Branch Protection Mechanisms

### 1. Pre-Push Hook Validation

The repository includes a sophisticated pre-push hook (`.git/hooks/pre-push`) that runs before any push to the public repository.

#### Branch Naming Validation

**Blocked Patterns:**
```bash
# Hyphen patterns
private-*, internal-*, client-*, secret-*
wip-private-*, customer-*, proprietary-*, confidential-*

# Underscore patterns
private_*, internal_*, client_*, secret_*
wip-private_*, customer_*, proprietary_*, confidential_*

# Explicit private branches
sqlite_haystack, private-feature, internal-docs, customer-data, private_tf
```

**Allowed Patterns:**
```bash
# Feature branches
feat/new-ui-component
feat/semantic-search
fix/memory-leak
docs/update-readme
refactor/cleanup-code
test/add-unit-tests

# Development branches
wip/experimental-feature
experimental/new-algorithm
```

#### Commit Message Validation

The hook scans commit messages for private markers:

**Blocked Markers:**
- `[PRIVATE]`, `[INTERNAL]`, `[CLIENT]`, `[DO-NOT-PUSH]`
- `[CONFIDENTIAL]`, `[CUSTOMER]`
- `private:`, `internal:`, `client:`, `secret:`, `confidential:`

#### Content Validation

**Sensitive Keywords Detected:**
- `customer_api_key`, `client_secret`, `private_key_prod`
- `internal_endpoint`, `zestic.*api`, `customer.*password`
- `truthforge`, `private.*cloud`, `internal.*api`

**File Pattern Validation:**
- Uses `.gitprivateignore` file for custom patterns
- Scans file contents for sensitive data
- Prevents accidental inclusion of private files

### 2. Repository-Level Protection

#### Main Branch Protection

The main branch has comprehensive protection rules:

- **Required Status Checks**: CI-native workflow must pass
- **Required Reviews**: At least 1 approval required for pull requests
- **Admin Enforcement**: Even admins must follow protection rules
- **Force Push Prevention**: Disabled to prevent history rewriting
- **Branch Deletion Prevention**: Disabled to prevent accidental deletion

#### Repository Settings

- **Merge Options**: Configured for squash, merge, and rebase
- **Auto-merge**: Disabled for manual review requirement
- **Branch Cleanup**: Auto-delete branches after merge

## Development Workflows

### Public Development Workflow

For open-source development:

1. **Create feature branch**:
   ```bash
   git checkout -b feat/new-feature
   ```

2. **Develop and commit**:
   ```bash
   git add .
   git commit -m "feat: add new feature"
   ```

3. **Push to public repository**:
   ```bash
   git push origin feat/new-feature
   ```

4. **Create pull request** and wait for review

### Private Development Workflow

For proprietary or sensitive development:

1. **Switch to private repository**:
   ```bash
   git remote set-url origin git@github.com:zestic-ai/terraphim-private.git
   ```

2. **Create private branch**:
   ```bash
   git checkout -b private-feature
   # or
   git checkout -b internal-api
   ```

3. **Develop normally**:
   ```bash
   git add .
   git commit -m "feat: add private feature"
   git push origin private-feature
   ```

4. **Switch back to public** (when ready):
   ```bash
   git remote set-url origin https://github.com/terraphim/terraphim-ai.git
   ```

### Hybrid Development Workflow

For projects that span both repositories:

1. **Configure both remotes**:
   ```bash
   git remote add private git@github.com:zestic-ai/terraphim-private.git
   ```

2. **Set branch-specific remotes**:
   ```bash
   git config branch.private-feature.remote private
   git config branch.private-feature.pushRemote private
   ```

3. **Develop in appropriate repository**:
   ```bash
   # For private work
   git checkout private-feature
   git push private private-feature

   # For public work
   git checkout feat/public-feature
   git push origin feat/public-feature
   ```

## Troubleshooting

### Common Error Messages

#### "Branch matches private pattern"
```bash
✗ Branch 'private_tf' matches private pattern '^private_'
Private branches should not be pushed to public remotes.
Push to private remote instead: git push private private_tf
```

**Solution:**
```bash
# Configure branch to push to private remote
git config branch.private_tf.remote private
git config branch.private_tf.pushRemote private

# Push to private repository
git push private private_tf
```

#### "Sensitive keyword found"
```bash
✗ Sensitive keyword 'truthforge' found in file changes
Remove sensitive content before pushing to public remote.
```

**Solution:**
- Remove or replace sensitive content
- Use private repository for sensitive development
- Update `.gitprivateignore` if it's a false positive

#### "Commit message contains private markers"
```bash
✗ Found commits with private markers:
1eccff1 [PRIVATE] add sensitive feature
Remove private markers or push to private remote instead.
```

**Solution:**
- Rewrite commit message without private markers
- Use `git commit --amend` to fix the message
- Or push to private repository

### Configuration Issues

#### Branch not configured for private remote
```bash
# Check current remote configuration
git remote -v

# Add private remote if missing
git remote add private git@github.com:zestic-ai/terraphim-private.git

# Configure branch for private remote
git config branch.branch-name.remote private
git config branch.branch-name.pushRemote private
```

#### Pre-push hook not running
```bash
# Check if hook exists and is executable
ls -la .git/hooks/pre-push

# Make executable if needed
chmod +x .git/hooks/pre-push

# Test hook manually
.git/hooks/pre-push origin https://github.com/terraphim/terraphim-ai.git
```

## Security Features

### Audit Logging

The pre-push hook logs all push attempts to `.git/push-audit.log`:

```bash
# View recent push attempts
tail -f .git/push-audit.log

# Example log entry
2025-01-08 19:33:22 - SUCCESS: Push to origin from feat/new-feature
2025-01-08 19:35:15 - BLOCKED: Push to origin from private_tf (private pattern)
```

### Monitoring and Alerts

- **Failed push attempts** are logged with timestamps and reasons
- **Sensitive content detection** is logged for security monitoring
- **Branch pattern violations** are tracked for compliance

### Configuration Management

#### .gitprivateignore File

Create a `.gitprivateignore` file to define custom file patterns:

```bash
# Example .gitprivateignore
**/private/**
**/internal/**
**/client-data/**
**/*.secret
**/config/production.*
```

#### Sensitive Keywords

Update sensitive keywords in `.git/hooks/pre-push`:

```bash
sensitive_keywords=(
    "customer_api_key"
    "client_secret"
    "private_key_prod"
    "internal_endpoint"
    "zestic.*api"
    "customer.*password"
    "truthforge"
    "private.*cloud"
    "internal.*api"
    # Add your custom keywords here
)
```

## Best Practices

### Branch Naming

- **Use descriptive names**: `feat/semantic-search` not `feat/ss`
- **Follow conventions**: `type/description` format
- **Avoid private patterns**: Use public repository for open-source work
- **Be consistent**: Follow team naming conventions

### Commit Messages

- **Use conventional format**: `type(scope): description`
- **Avoid private markers**: Don't use `[PRIVATE]`, `[INTERNAL]`, etc.
- **Be descriptive**: Explain what the commit does
- **Keep it concise**: One line summary, detailed body if needed

### Repository Management

- **Use appropriate repository**: Public for open-source, private for proprietary
- **Configure remotes properly**: Set branch-specific remotes when needed
- **Keep repositories in sync**: Regular updates between public and private
- **Document private features**: Use private repository for sensitive documentation

### Security Considerations

- **Review before pushing**: Always check what you're pushing
- **Use private repository**: For any sensitive or proprietary code
- **Update patterns**: Add new sensitive keywords as needed
- **Monitor logs**: Check push audit logs regularly
- **Train team members**: Ensure everyone understands the protection system

## Advanced Configuration

### Custom Hook Configuration

The pre-push hook can be customized by editing `.git/hooks/pre-push`:

```bash
# Add new private branch patterns
private_branch_patterns=(
    "^private-"
    "^private_"
    "^internal-"
    "^internal_"
    "^custom-pattern-"  # Add your pattern here
)

# Add new sensitive keywords
sensitive_keywords=(
    "customer_api_key"
    "client_secret"
    "custom_sensitive_key"  # Add your keyword here
)
```

### Git Configuration

Set up global or repository-specific configurations:

```bash
# Global configuration
git config --global branch.autoSetupRemote true

# Repository-specific configuration
git config branch.private-feature.remote private
git config branch.private-feature.pushRemote private
```

### IDE Integration

Most IDEs can be configured to work with the dual-repository setup:

- **VS Code**: Use GitLens extension for remote management
- **IntelliJ**: Configure multiple remotes in VCS settings
- **Vim/Neovim**: Use fugitive.vim for Git operations

## Compliance and Legal

### Open Source Compliance

- **License compatibility**: Ensure all public code is properly licensed
- **Attribution requirements**: Include proper copyright notices
- **Dependency management**: Track and document all dependencies

### Security Compliance

- **Data protection**: Ensure no sensitive data in public repository
- **Access controls**: Properly manage repository access
- **Audit trails**: Maintain logs for compliance requirements

### Client Confidentiality

- **Client data**: Never commit client-specific data to public repository
- **Proprietary algorithms**: Keep proprietary code in private repository
- **NDA compliance**: Ensure all work respects non-disclosure agreements

---

This comprehensive branch protection system ensures that Terraphim AI maintains the highest security standards while enabling efficient development workflows across both public and private repositories.