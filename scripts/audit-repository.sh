#!/bin/bash
#
# Repository Security Audit Script for Terraphim AI
# Performs periodic security checks to detect potential private data leaks
# and ensure repository hygiene
#
# Usage: ./scripts/audit-repository.sh [--fix] [--report]
#

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color

# Script configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
AUDIT_LOG="$REPO_ROOT/.git/security-audit.log"
REPORT_FILE="$REPO_ROOT/.git/security-audit-$(date +%Y%m%d-%H%M%S).txt"

# Counters
ISSUES_FOUND=0
WARNINGS_FOUND=0
CHECKS_PASSED=0

# Function to print colored output
print_status() {
    local status=$1
    local message=$2
    if [ "$status" = "SUCCESS" ]; then
        echo -e "${GREEN}✓${NC} $message"
        ((CHECKS_PASSED++))
    elif [ "$status" = "FAIL" ]; then
        echo -e "${RED}✗${NC} $message"
        ((ISSUES_FOUND++))
    elif [ "$status" = "WARN" ]; then
        echo -e "${YELLOW}⚠${NC} $message"
        ((WARNINGS_FOUND++))
    elif [ "$status" = "INFO" ]; then
        echo -e "${BLUE}ℹ${NC} $message"
    elif [ "$status" = "HEADER" ]; then
        echo -e "${CYAN}$message${NC}"
    else
        echo -e "$message"
    fi
}

# Function to log audit results
log_audit() {
    local result=$1
    local check=$2
    local details=${3:-""}
    echo "$(date '+%Y-%m-%d %H:%M:%S') - $result: $check - $details" >> "$AUDIT_LOG"
}

# Function to check git repository status
check_git_status() {
    print_status "HEADER" "=== Git Repository Status ==="
    
    if ! git rev-parse --git-dir >/dev/null 2>&1; then
        print_status "FAIL" "Not in a git repository"
        log_audit "FAIL" "Git Status" "Not a git repository"
        return 1
    fi
    
    print_status "SUCCESS" "Git repository detected"
    
    # Check for uncommitted changes
    if ! git diff-index --quiet HEAD -- 2>/dev/null; then
        print_status "WARN" "Uncommitted changes detected"
        log_audit "WARN" "Git Status" "Uncommitted changes present"
        git status --porcelain | head -10 | sed 's/^/  /'
    else
        print_status "SUCCESS" "Working directory clean"
        log_audit "SUCCESS" "Git Status" "Working directory clean"
    fi
    
    # Check current branch
    local current_branch=$(git symbolic-ref --short HEAD 2>/dev/null || echo "detached")
    print_status "INFO" "Current branch: $current_branch"
    
    # Check remotes
    print_status "INFO" "Configured remotes:"
    git remote -v | sed 's/^/  /'
    
    echo ""
}

# Function to scan for potential private data leaks
scan_private_leaks() {
    print_status "HEADER" "=== Scanning for Private Data Leaks ==="
    
    # Check branch names in history
    print_status "INFO" "Checking branch names in history..."
    local private_branches=$(git branch -a | grep -E '(private-|internal-|client-|secret-)' || true)
    
    if [ -n "$private_branches" ]; then
        print_status "WARN" "Found branches with private naming patterns:"
        echo "$private_branches" | sed 's/^/  /'
        log_audit "WARN" "Private Branches" "$(echo "$private_branches" | wc -l) private branches found"
    else
        print_status "SUCCESS" "No private branch names found"
        log_audit "SUCCESS" "Private Branches" "No private branches detected"
    fi
    
    # Scan commit messages for private markers
    print_status "INFO" "Scanning commit messages for private markers..."
    local private_commits=$(git log --oneline --all | grep -i -E '\[(PRIVATE|INTERNAL|CLIENT|CONFIDENTIAL)\]|private:|internal:|client:' | head -10 || true)
    
    if [ -n "$private_commits" ]; then
        print_status "WARN" "Found commits with private markers:"
        echo "$private_commits" | sed 's/^/  /'
        log_audit "WARN" "Private Commits" "$(echo "$private_commits" | wc -l) commits with private markers"
    else
        print_status "SUCCESS" "No private markers found in commit messages"
        log_audit "SUCCESS" "Private Commits" "No private markers detected"
    fi
    
    # Check for sensitive files in current state
    print_status "INFO" "Checking for sensitive file patterns..."
    if [ -f ".gitprivateignore" ]; then
        local sensitive_files=""
        while IFS= read -r pattern || [ -n "$pattern" ]; do
            # Skip comments, empty lines, and exclusions
            [[ "$pattern" =~ ^#.*$ || -z "$pattern" || "$pattern" =~ ^!.* ]] && continue
            
            local matches=$(find . -path "./.git" -prune -o -name "$pattern" -type f -print 2>/dev/null || true)
            if [ -n "$matches" ]; then
                sensitive_files="$sensitive_files\n$pattern: $matches"
            fi
        done < .gitprivateignore
        
        if [ -n "$sensitive_files" ]; then
            print_status "WARN" "Found files matching private patterns:"
            echo -e "$sensitive_files" | head -20 | sed 's/^/  /'
            log_audit "WARN" "Sensitive Files" "Files matching private patterns found"
        else
            print_status "SUCCESS" "No sensitive file patterns found"
            log_audit "SUCCESS" "Sensitive Files" "No sensitive files detected"
        fi
    else
        print_status "WARN" ".gitprivateignore file not found"
        log_audit "WARN" "Configuration" ".gitprivateignore missing"
    fi
    
    echo ""
}

# Function to audit security configuration
audit_security_config() {
    print_status "HEADER" "=== Security Configuration Audit ==="
    
    # Check pre-push hook
    if [ -x ".git/hooks/pre-push" ]; then
        print_status "SUCCESS" "Pre-push hook is installed and executable"
        log_audit "SUCCESS" "Hooks" "Pre-push hook configured"
        
        # Check if it contains private leak prevention
        if grep -q "private-to-public leak prevention" ".git/hooks/pre-push" 2>/dev/null; then
            print_status "SUCCESS" "Pre-push hook includes leak prevention logic"
        else
            print_status "WARN" "Pre-push hook may not include leak prevention"
            log_audit "WARN" "Hooks" "Pre-push hook missing leak prevention"
        fi
    else
        print_status "FAIL" "Pre-push hook is not installed or not executable"
        log_audit "FAIL" "Hooks" "Pre-push hook missing"
    fi
    
    # Check pre-commit hook
    if [ -x ".git/hooks/pre-commit" ]; then
        print_status "SUCCESS" "Pre-commit hook is installed and executable"
        log_audit "SUCCESS" "Hooks" "Pre-commit hook configured"
    else
        print_status "WARN" "Pre-commit hook is not installed or not executable"
        log_audit "WARN" "Hooks" "Pre-commit hook missing"
    fi
    
    # Check validation script
    if [ -x "scripts/validate-push.sh" ]; then
        print_status "SUCCESS" "Validation script is available and executable"
        log_audit "SUCCESS" "Scripts" "Validation script configured"
    else
        print_status "FAIL" "Validation script is missing or not executable"
        log_audit "FAIL" "Scripts" "Validation script missing"
    fi
    
    # Check .gitprivateignore
    if [ -f ".gitprivateignore" ]; then
        local pattern_count=$(grep -v '^#' .gitprivateignore | grep -v '^$' | wc -l)
        print_status "SUCCESS" ".gitprivateignore exists with $pattern_count patterns"
        log_audit "SUCCESS" "Configuration" ".gitprivateignore configured with $pattern_count patterns"
    else
        print_status "FAIL" ".gitprivateignore file is missing"
        log_audit "FAIL" "Configuration" ".gitprivateignore missing"
    fi
    
    # Check SECURITY.md
    if [ -f "SECURITY.md" ]; then
        print_status "SUCCESS" "SECURITY.md documentation exists"
        log_audit "SUCCESS" "Documentation" "SECURITY.md present"
    else
        print_status "WARN" "SECURITY.md documentation is missing"
        log_audit "WARN" "Documentation" "SECURITY.md missing"
    fi
    
    echo ""
}

# Function to check recent activity
check_recent_activity() {
    print_status "HEADER" "=== Recent Activity Analysis ==="
    
    # Check recent commits
    print_status "INFO" "Recent commits (last 10):"
    git log --oneline -10 | sed 's/^/  /'
    
    # Check recent pushes from audit log
    if [ -f ".git/push-audit.log" ]; then
        print_status "INFO" "Recent push attempts:"
        tail -10 .git/push-audit.log 2>/dev/null | sed 's/^/  /' || true
    else
        print_status "INFO" "No push audit log found"
    fi
    
    # Check for recent validation runs
    if [ -f ".git/validation-audit.log" ]; then
        print_status "INFO" "Recent validation runs:"
        tail -5 .git/validation-audit.log 2>/dev/null | sed 's/^/  /' || true
    else
        print_status "INFO" "No validation audit log found"
    fi
    
    echo ""
}

# Function to scan for secrets in history
scan_secrets_history() {
    print_status "HEADER" "=== Historical Secret Scan ==="
    
    local secret_patterns=(
        "api_key.*=.*['\"][^'\"]{10,}['\"]"
        "password.*=.*['\"][^'\"]{8,}['\"]"
        "secret.*=.*['\"][^'\"]{10,}['\"]"
        "token.*=.*['\"][^'\"]{20,}['\"]"
        "AKIA[0-9A-Z]{16}"
        "sk_live_[0-9a-zA-Z]{24}"
        "ghp_[0-9a-zA-Z]{36}"
    )
    
    local secrets_found=false
    
    for pattern in "${secret_patterns[@]}"; do
        print_status "INFO" "Scanning for pattern: $pattern"
        
        # Scan recent commits (last 100)
        local matches=$(git log --all --grep="$pattern" --oneline -100 2>/dev/null || true)
        if [ -n "$matches" ]; then
            print_status "WARN" "Potential secrets found in commit messages:"
            echo "$matches" | head -5 | sed 's/^/  /'
            secrets_found=true
        fi
        
        # Scan file changes in recent commits
        local file_matches=$(git log --all -p -100 | grep -E "$pattern" | head -5 || true)
        if [ -n "$file_matches" ]; then
            print_status "WARN" "Potential secrets found in file changes:"
            echo "$file_matches" | sed 's/^/  /'
            secrets_found=true
        fi
    done
    
    if [ "$secrets_found" = false ]; then
        print_status "SUCCESS" "No obvious secrets found in recent history"
        log_audit "SUCCESS" "Secret Scan" "No secrets detected in recent history"
    else
        print_status "FAIL" "Potential secrets found - manual review required"
        log_audit "FAIL" "Secret Scan" "Potential secrets detected"
    fi
    
    echo ""
}

# Function to generate recommendations
generate_recommendations() {
    print_status "HEADER" "=== Security Recommendations ==="
    
    local recommendations=()
    
    # Check if security is properly configured
    if [ ! -x ".git/hooks/pre-push" ]; then
        recommendations+=("Install and configure pre-push hook to prevent private data leaks")
    fi
    
    if [ ! -f ".gitprivateignore" ]; then
        recommendations+=("Create .gitprivateignore file with private file patterns")
    fi
    
    if [ ! -f "SECURITY.md" ]; then
        recommendations+=("Create SECURITY.md documentation for security procedures")
    fi
    
    if [ $ISSUES_FOUND -gt 0 ]; then
        recommendations+=("Address the $ISSUES_FOUND critical security issues found above")
    fi
    
    if [ $WARNINGS_FOUND -gt 3 ]; then
        recommendations+=("Review and address the $WARNINGS_FOUND warnings found above")
    fi
    
    # Check git configuration
    local push_default=$(git config push.default 2>/dev/null || echo "not set")
    if [ "$push_default" != "simple" ] && [ "$push_default" != "current" ]; then
        recommendations+=("Configure safe git push defaults: git config push.default simple")
    fi
    
    if [ ${#recommendations[@]} -eq 0 ]; then
        print_status "SUCCESS" "No immediate recommendations - security configuration looks good!"
    else
        print_status "INFO" "Security recommendations:"
        for rec in "${recommendations[@]}"; do
            echo "  • $rec"
        done
    fi
    
    echo ""
}

# Function to generate report
generate_report() {
    cat > "$REPORT_FILE" << EOF
TERRAPHIM AI SECURITY AUDIT REPORT
==================================
Date: $(date)
Repository: $(pwd)
Auditor: $(git config user.name || echo "Unknown") <$(git config user.email || echo "unknown@example.com")>

SUMMARY
-------
• Checks Passed: $CHECKS_PASSED
• Issues Found: $ISSUES_FOUND  
• Warnings: $WARNINGS_FOUND

REPOSITORY STATUS
-----------------
• Current Branch: $(git symbolic-ref --short HEAD 2>/dev/null || echo "detached")
• Last Commit: $(git log -1 --oneline 2>/dev/null || echo "No commits")
• Working Directory: $(git diff-index --quiet HEAD -- 2>/dev/null && echo "Clean" || echo "Has changes")

SECURITY CONFIGURATION
----------------------
• Pre-push Hook: $([ -x ".git/hooks/pre-push" ] && echo "✓ Installed" || echo "✗ Missing")
• Pre-commit Hook: $([ -x ".git/hooks/pre-commit" ] && echo "✓ Installed" || echo "✗ Missing") 
• Validation Script: $([ -x "scripts/validate-push.sh" ] && echo "✓ Available" || echo "✗ Missing")
• .gitprivateignore: $([ -f ".gitprivateignore" ] && echo "✓ Present" || echo "✗ Missing")
• SECURITY.md: $([ -f "SECURITY.md" ] && echo "✓ Present" || echo "✗ Missing")

PRIVATE DATA SCAN
-----------------
• Private Branches: $(git branch -a | grep -E '(private-|internal-|client-|secret-)' | wc -l || echo "0") found
• Private Commits: $(git log --oneline --all | grep -i -E '\[(PRIVATE|INTERNAL|CLIENT)\]' | wc -l || echo "0") found
• Sensitive Files: Scanned against .gitprivateignore patterns

RECOMMENDATIONS
---------------
$(if [ $ISSUES_FOUND -eq 0 ] && [ $WARNINGS_FOUND -lt 3 ]; then
    echo "✅ Security posture is good"
    echo "✅ Continue following security best practices"
else
    echo "⚠️  Address the issues and warnings identified above"
    echo "⚠️  Review and update security configurations"
fi)

NEXT STEPS
----------
1. Review this report and address any critical issues
2. Run './scripts/validate-push.sh' before public pushes
3. Ensure team members are trained on security procedures
4. Schedule regular security audits (monthly recommended)

For detailed logs, see: $AUDIT_LOG
Report generated by: $(basename "$0")
EOF

    print_status "SUCCESS" "Audit report generated: $REPORT_FILE"
}

# Function to fix common issues
fix_issues() {
    print_status "HEADER" "=== Fixing Common Issues ==="
    
    local fixes_applied=0
    
    # Make scripts executable
    if [ -f "scripts/validate-push.sh" ] && [ ! -x "scripts/validate-push.sh" ]; then
        chmod +x scripts/validate-push.sh
        print_status "SUCCESS" "Made validation script executable"
        ((fixes_applied++))
    fi
    
    if [ -f "scripts/configure-git-security.sh" ] && [ ! -x "scripts/configure-git-security.sh" ]; then
        chmod +x scripts/configure-git-security.sh
        print_status "SUCCESS" "Made git security configuration script executable"
        ((fixes_applied++))
    fi
    
    if [ -f "scripts/audit-repository.sh" ] && [ ! -x "scripts/audit-repository.sh" ]; then
        chmod +x scripts/audit-repository.sh
        print_status "SUCCESS" "Made audit script executable"
        ((fixes_applied++))
    fi
    
    # Configure basic git security settings
    local current_push_default=$(git config push.default 2>/dev/null || echo "not set")
    if [ "$current_push_default" != "simple" ]; then
        git config push.default simple
        print_status "SUCCESS" "Configured safe git push default"
        ((fixes_applied++))
    fi
    
    print_status "INFO" "Applied $fixes_applied automatic fixes"
    
    if [ $fixes_applied -eq 0 ]; then
        print_status "INFO" "No automatic fixes needed"
    fi
    
    echo ""
}

# Main audit function
main() {
    local fix_issues=false
    local generate_report_file=false
    
    # Parse arguments
    while [[ $# -gt 0 ]]; do
        case $1 in
            --fix)
                fix_issues=true
                shift
                ;;
            --report)
                generate_report_file=true
                shift
                ;;
            --help|-h)
                cat << EOF
Terraphim AI Repository Security Audit Script
==============================================

Usage: $0 [OPTIONS]

Options:
  --fix         Automatically fix common security configuration issues
  --report      Generate a detailed audit report file
  --help, -h    Show this help message

This script performs comprehensive security checks including:
• Git repository status and configuration
• Private data leak detection 
• Security configuration validation
• Recent activity analysis
• Historical secret scanning
• Security recommendations

The audit results are logged to .git/security-audit.log
Use --report to generate a detailed report file.
EOF
                exit 0
                ;;
            *)
                print_status "FAIL" "Unknown option: $1"
                print_status "INFO" "Use --help for usage information"
                exit 1
                ;;
        esac
    done
    
    echo "🔍 Terraphim AI Repository Security Audit"
    echo "=========================================="
    print_status "INFO" "Starting security audit at $(date)"
    print_status "INFO" "Repository: $(pwd)"
    print_status "INFO" "Audit log: $AUDIT_LOG"
    echo ""
    
    # Change to repository root
    cd "$REPO_ROOT"
    
    # Log audit start
    log_audit "START" "Security Audit" "Audit started"
    
    # Run audit checks
    check_git_status
    scan_private_leaks  
    audit_security_config
    check_recent_activity
    scan_secrets_history
    generate_recommendations
    
    # Apply fixes if requested
    if [ "$fix_issues" = true ]; then
        fix_issues
    fi
    
    # Generate report if requested
    if [ "$generate_report_file" = true ]; then
        generate_report
        echo ""
    fi
    
    # Summary
    print_status "HEADER" "=== Audit Summary ==="
    print_status "INFO" "Checks Passed: $CHECKS_PASSED"
    if [ $ISSUES_FOUND -gt 0 ]; then
        print_status "FAIL" "Critical Issues: $ISSUES_FOUND"
    fi
    if [ $WARNINGS_FOUND -gt 0 ]; then
        print_status "WARN" "Warnings: $WARNINGS_FOUND"
    fi
    
    # Log audit completion
    log_audit "COMPLETE" "Security Audit" "Passed: $CHECKS_PASSED, Issues: $ISSUES_FOUND, Warnings: $WARNINGS_FOUND"
    
    echo ""
    if [ $ISSUES_FOUND -eq 0 ]; then
        print_status "SUCCESS" "Security audit completed successfully!"
        print_status "INFO" "Repository security posture is acceptable"
        exit 0
    else
        print_status "FAIL" "Security audit found critical issues!"
        print_status "INFO" "Please address the issues above before proceeding"
        exit 1
    fi
}

# Run main function
main "$@"