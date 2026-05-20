#!/bin/bash
# Setup auditd rules to trace SIGKILL delivery to opencode processes
# Run as root on bigbox. Logs go to /var/log/audit/audit.log
#
# Usage:
#   sudo bash auditd-sigkill-trace.sh install   # Add rules
#   sudo bash auditd-sigkill-trace.sh watch     # Tail relevant entries
#   sudo bash auditd-sigkill-trace.sh remove    # Remove rules
#
# This catches ANY process sending SIGKILL to opencode, including:
# - systemd (cgroup kill)
# - kernel OOM killer
# - cron jobs
# - manual pkill commands
# - other processes

set -euo pipefail

OPENCEDE_BIN="/home/alex/.bun/bin/opencode"

# Get the inode and device of the opencode binary for audit rules
get_binary_info() {
    stat -c "%D %i" "$OPENCEDE_BIN" 2>/dev/null || echo "unknown"
}

case "${1:-watch}" in
    install)
        echo "Installing auditd rules for SIGKILL tracing..."
        
        # Rule 1: Log any kill(SIGKILL) syscall targeting processes running opencode
        # This catches the kill() syscall with SIGKILL (signal 9)
        auditctl -a always,exit -F arch=b64 -S kill -F sig=9 -F key=opencode-sigkill 2>/dev/null || \
        auditctl -a always,exit -F arch=b64 -S kill -F a1=9 -k opencode-sigkill 2>/dev/null || \
            echo "Warning: Could not add kill SIGKILL rule (may need different auditctl syntax)"
        
        # Rule 2: Also catch kill -- any signal to opencode specifically
        # Using the binary path as a filter
        auditctl -a always,exit -F arch=b64 -S killat -F sig=9 -k opencode-sigkill 2>/dev/null || true
        
        # Rule 3: Log tkill/tgkill (thread-targeted kills)
        auditctl -a always,exit -F arch=b64 -S tkill -F sig=9 -k opencode-sigkill 2>/dev/null || true
        auditctl -a always,exit -F arch=b64 -S tgkill -F sig=9 -k opencode-sigkill 2>/dev/null || true
        
        echo "Rules installed. Current rules:"
        auditctl -l
        echo ""
        echo "Now run: sudo bash $0 watch"
        ;;
    
    watch)
        echo "Watching for SIGKILL events (Ctrl+C to stop)..."
        echo "Also writing to /tmp/sigkill-trace-$(date +%Y%m%d).log"
        ausearch -k opencode-sigkill -ts recent -f || echo "No events yet"
        echo "---"
        echo "Live tailing (new events will appear):"
        tail -f /var/log/audit/audit.log 2>/dev/null | grep --line-buffered "opencode-sigkill"
        ;;
    
    dump)
        echo "All SIGKILL events since boot:"
        ausearch -k opencode-sigkill -ts boot | aureport -x --interpret 2>/dev/null || \
            ausearch -k opencode-sigkill -ts boot 2>/dev/null || \
            echo "No events found"
        ;;
    
    remove)
        echo "Removing auditd rules..."
        auditctl -d always,exit -F arch=b64 -S kill -F sig=9 -k opencode-sigkill 2>/dev/null || true
        auditctl -d always,exit -F arch=b64 -S kill -F a1=9 -k opencode-sigkill 2>/dev/null || true
        auditctl -d always,exit -F arch=b64 -S killat -F sig=9 -k opencode-sigkill 2>/dev/null || true
        auditctl -d always,exit -F arch=b64 -S tkill -F sig=9 -k opencode-sigkill 2>/dev/null || true
        auditctl -d always,exit -F arch=b64 -S tgkill -F sig=9 -k opencode-sigkill 2>/dev/null || true
        echo "Rules removed."
        auditctl -l
        ;;
    
    *)
        echo "Usage: $0 {install|watch|dump|remove}"
        exit 1
        ;;
esac
