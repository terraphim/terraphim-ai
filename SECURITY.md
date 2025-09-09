# Security Policy for Terraphim AI

## Overview

This document outlines the security policies and procedures for the Terraphim AI project, specifically focusing on preventing private or sensitive information from accidentally leaking into public repositories.

## 🔒 Repository Structure

This project maintains a dual-repository approach:

- **Public Repository**: `origin` - `terraphim/terraphim-ai` - Contains open-source code
- **Private Repository**: `private` - `zestic-ai/terraphim-private` - Contains proprietary code, client data, and sensitive information

## 🛡️ Private-to-Public Leak Prevention

### Automated Protection Layers

Our security system implements multiple layers of protection:

1. **Enhanced Pre-push Hook** (`.git/hooks/pre-push`)
2. **Comprehensive Validation Script** (`scripts/validate-push.sh`)
3. **Private File Pattern Detection** (`.gitprivateignore`)
4. **Pre-commit Content Scanning** (`.git/hooks/pre-commit`)
5. **Branch Naming Enforcement**
6. **Commit Message Scanning**

### Branch Naming Conventions

#### ❌ Private Branch Patterns (DO NOT push to public)
Branches matching these patterns are automatically blocked from public remotes:

- `private-*` - Private features or experiments
- `internal-*` - Internal company features  
- `client-*` - Client-specific implementations
- `secret-*` - Sensitive or confidential work
- `wip-private-*` - Work-in-progress private features
- `customer-*` - Customer-specific code
- `proprietary-*` - Proprietary algorithms or features
- `confidential-*` - Confidential research or development

#### ✅ Public Branch Patterns (Safe for public)
- `feature-*` - Public feature development
- `fix-*` - Bug fixes for public release
- `docs-*` - Documentation updates
- `release-*` - Release preparation
- `main` / `master` - Main development branch
- `develop` - Development integration branch

#### ⚠️ Review Required Patterns
These branches require manual confirmation before pushing to public:
- `thinking-*` - Experimental or brainstorming work
- `experimental-*` - Proof-of-concept implementations
- `wip-*` - Work-in-progress (not private-specific)

### Commit Message Guidelines

#### ❌ Private Markers (Prevented from public push)
- `[PRIVATE]` - Private feature or sensitive change
- `[INTERNAL]` - Internal company use only
- `[CLIENT]` - Client-specific implementation
- `[DO-NOT-PUSH]` - Explicitly marked for private only
- `[CONFIDENTIAL]` - Confidential information
- `[CUSTOMER]` - Customer-specific changes
- `private:` - Private feature prefix
- `internal:` - Internal feature prefix
- `client:` - Client-specific prefix
- `secret:` - Sensitive change prefix
- `confidential:` - Confidential prefix

#### ✅ Good Public Commit Messages
```
feat: add new search functionality
fix: resolve memory leak in indexer
docs: update API documentation
style: format code with rustfmt
test: add unit tests for haystack module
chore: update dependencies
```

### File and Directory Patterns

The `.gitprivateignore` file contains patterns for files that should never be pushed to public repositories. Key categories include:

- **Client/Customer Data**: `client-*`, `customer-*`, `customers/`
- **Private Configurations**: `*private*.json`, `*internal*.json`, `config/private/`
- **API Keys/Secrets**: `*api_key*`, `*secret_key*`, `.env.production`
- **Internal Documentation**: `docs/private/`, `*-private.md`, `PRIVATE*.md`
- **Proprietary Code**: `src/private/`, `crates/private_*`
- **And many more...** (see `.gitprivateignore` for complete list)

## 🚀 Development Workflow

### Setting Up Your Environment

1. **Clone the repository**:
   ```bash
   git clone git@github.com:terraphim/terraphim-ai.git
   cd terraphim-ai
   ```

2. **Add private remote**:
   ```bash
   git remote add private git@github.com:zestic-ai/terraphim-private.git
   ```

3. **Configure git security settings**:
   ```bash
   ./scripts/configure-git-security.sh --apply
   ```

4. **Verify protection is active**:
   ```bash
   ./scripts/validate-push.sh
   ```

### Daily Development

#### Creating New Branches

**For public work**:
```bash
git checkout -b feature-new-search-algorithm
# Work on public features...
git push origin feature-new-search-algorithm  # Safe
```

**For private work**:
```bash
git checkout -b private-client-integration
# Work on sensitive features...
git push private private-client-integration   # Safe
```

#### Committing Changes

**Public commits**:
```bash
git commit -m "feat: implement BM25 scoring algorithm"
```

**Private commits** (will be blocked from public):
```bash
git commit -m "[PRIVATE] client: add Zestic-specific configuration"
```

### Pushing Changes

#### Safe Commands
```bash
# Validate before pushing
git validate-push

# Push safely with validation
git safe-push

# Push explicitly to public
git push-public

# Push explicitly to private
git push-private
```

#### Manual Pushing
```bash
# For public work
git push origin feature-branch

# For private work  
git push private private-branch
```

## 🔍 Validation and Testing

### Manual Validation

Run the validation script before any public push:
```bash
./scripts/validate-push.sh [branch-name] [remote-name]
```

This checks for:
- Branch naming violations
- Private markers in commits
- Sensitive file patterns
- Sensitive keywords in content
- Internal email domains in commits

### Testing Protection

Test the protection system:

1. **Create a test private branch**:
   ```bash
   git checkout -b private-test-protection
   echo "customer_api_key=secret123" > test-sensitive.txt
   git add test-sensitive.txt
   git commit -m "[PRIVATE] test: add sensitive data"
   ```

2. **Try to push to public** (should fail):
   ```bash
   git push origin private-test-protection
   ```

3. **Push to private** (should succeed):
   ```bash
   git push private private-test-protection
   ```

4. **Clean up**:
   ```bash
   git checkout main
   git branch -D private-test-protection
   ```

## 🔧 Maintenance and Monitoring

### Audit Scripts

Run periodic security audits:

```bash
# Check for potential leaks in public repo
./scripts/audit-repository.sh

# Generate security report
./scripts/validate-push.sh --report

# Review push history
cat .git/push-audit.log
```

### Log Files

The system maintains several log files:
- `.git/push-audit.log` - All push attempts and results
- `.git/validation-audit.log` - Detailed validation logs
- `.git/validation-report-*.txt` - Validation reports

### Updating Protection Rules

To modify protection patterns:

1. **Update `.gitprivateignore`** for file patterns
2. **Edit `.git/hooks/pre-push`** for branch/commit rules
3. **Modify `scripts/validate-push.sh`** for validation logic
4. **Test changes thoroughly** before deployment

## 📚 Recovery Procedures

### If Private Data is Accidentally Pushed to Public

**⚠️ IMMEDIATE ACTIONS:**

1. **DO NOT PANIC** - Quick action can minimize exposure
2. **Notify the team immediately**
3. **Follow these steps**:

```bash
# 1. Force push to remove sensitive commits (if just pushed)
git reset --hard HEAD~[number-of-commits]
git push --force-with-lease origin branch-name

# 2. If the data has been there longer, use git-filter-repo
pip install git-filter-repo
git filter-repo --path sensitive-file --invert-paths

# 3. Rotate any exposed secrets immediately
# 4. Consider repository as compromised
```

**IMPORTANT**: 
- GitHub keeps dangling commits for a long time
- Force pushes may not be sufficient if others have pulled
- Consider creating a new repository if sensitive data was exposed
- All exposed secrets must be rotated immediately

### If Protection System Fails

1. **Report the issue** immediately to the security team
2. **Review recent commits** for sensitive content
3. **Update protection rules** to prevent future occurrences
4. **Test the fix** thoroughly
5. **Document the incident** for future reference

## 🏗️ Technical Implementation Details

### Pre-push Hook Flow
1. Identify target remote (public vs private)
2. Check branch naming conventions
3. Scan commit messages for private markers
4. Validate against `.gitprivateignore` patterns
5. Scan file contents for sensitive keywords
6. Check commit metadata (author emails)
7. Prompt for confirmation if needed
8. Log all attempts

### Validation Script Features
- Comprehensive security scanning
- Detailed reporting
- CI/CD integration ready
- Configurable patterns and rules
- Audit trail generation

### Git Configuration
- Branch-specific push remotes
- Safe push aliases
- Merge and rebase policies
- URL restrictions

## 📞 Contact and Support

### Security Team
- **Primary Contact**: Security Team Lead
- **Email**: security@terraphim.ai
- **Slack**: #security-alerts

### Emergency Response
- **Immediate Issues**: Contact security team directly
- **After Hours**: Use emergency contact procedures
- **Public Exposure**: Follow incident response plan

### Getting Help
- **Documentation**: This file and `scripts/--help`
- **Setup Issues**: Run `./scripts/configure-git-security.sh`
- **Validation Problems**: Run `./scripts/validate-push.sh --help`
- **Questions**: Ask in #development Slack channel

## 📈 Compliance and Standards

### Internal Standards
- All private data must be kept in private repository
- Public repository must not contain customer information
- API keys and secrets must never be committed
- Client-specific code must be properly segregated

### Regular Reviews
- Monthly security audits
- Quarterly protection rule updates
- Annual security training for developers
- Continuous monitoring of protection effectiveness

### Metrics and Reporting
- Track prevention effectiveness
- Monitor false positive rates
- Measure developer compliance
- Report security incidents

---

## Quick Reference Card

### ✅ Safe Practices
- Use proper branch naming (`feature-*`, `fix-*`)
- Clean commit messages without sensitive info
- Push public work to `origin`
- Push private work to `private`
- Run `git safe-push` for validation

### ❌ Dangerous Practices
- Private branch names to public repos
- Commit messages with `[PRIVATE]` to public
- Files matching `.gitprivateignore` patterns
- API keys or secrets in any commits
- Bypassing validation hooks

### 🔧 Key Commands
```bash
git safe-push                    # Validate and push
git validate-push               # Check without pushing
./scripts/configure-git-security.sh --apply
./scripts/validate-push.sh branch-name remote-name
```

### 🚨 Emergency Commands
```bash
git reset --hard HEAD~1         # Undo last commit
git push --force-with-lease     # Force push safely
git filter-repo --help          # Clean repository history
```

---

**Remember**: Security is everyone's responsibility. When in doubt, ask for help or push to the private repository first.