#!/bin/bash
# Diagnostic wrapper for opencode to trace SIGKILL source
# Usage: opencode-strace-wrapper <original_args...>
#
# This wrapper runs opencode under strace to capture all signal deliveries.
# Output goes to /tmp/opencode-strace-<timestamp>-<pid>.log
#
# Install: 
#   cp opencode-strace-wrapper.sh /home/alex/.bun/bin/opencode-strace-wrapper
#   chmod +x /home/alex/.bun/bin/opencode-strace-wrapper
#
# Then in agent config, change:
#   cli_tool = "/home/alex/.bun/bin/opencode"
# to:
#   cli_tool = "/home/alex/.bun/bin/opencode-strace-wrapper"

set -euo pipefail

TIMESTAMP=$(date +%Y%m%d-%H%M%S)
PID=$$
LOGDIR="/tmp/opencode-strace"
mkdir -p "$LOGDIR"

LOGFILE="${LOGDIR}/trace-${TIMESTAMP}-${PID}.log"

# Log basic info
{
    echo "=== opencode-strace-wrapper ==="
    echo "PID: $$"
    echo "PPID: $PPID"
    echo "Timestamp: ${TIMESTAMP}"
    echo "Args: $*"
    echo "UID: $(id -u)"
    echo "Cgroup:"
    cat /proc/self/cgroup 2>/dev/null || echo "(none)"
    echo "Memory limits:"
    cat /proc/self/memory_limit 2>/dev/null || echo "(none)"
    echo "RLIMIT_AS:"
    prlimit --pid $$ --as 2>/dev/null || echo "(prlimit not available)"
    echo "RLIMIT_CPU:"
    prlimit --pid $$ --cpu 2>/dev/null || echo "(prlimit not available)"
    echo ""
} > "$LOGFILE"

# Run opencode under strace, tracing only signals
# -e trace=signal: trace all signal-related syscalls
# -f: follow child processes (opencode spawns node/bun children)
# -o: output to log file (append)
# -tt: timestamp with microseconds
exec strace -e trace=signal -f -tt -o "${LOGDIR}/signals-${TIMESTAMP}-${PID}.log" \
    /home/alex/.bun/bin/opencode "$@" 2>>"${LOGDIR}/stderr-${TIMESTAMP}-${PID}.log"
