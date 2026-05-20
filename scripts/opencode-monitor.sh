#!/bin/bash
# Monitor opencode processes for SIGKILL events
# Uses /proc/PID/status to detect when processes receive signals
# Run: nohup bash opencode-monitor.sh > /tmp/opencode-monitor.log 2>&1 &
#
# This script polls all opencode processes every second and logs:
# - New opencode processes appearing
# - Process exit (with exit code and signal)
# - Process state changes (e.g., to "Z" zombie)
# - Memory usage changes

set -uo pipefail

POLL_INTERVAL=1
LOGFILE="/tmp/opencode-monitor.log"
KNOWN_PIDS=""  # Space-separated list of known opencode PIDs
ITERATION=0

log() {
    echo "$(date '+%Y-%m-%d %H:%M:%S.%3N') $*" >> "$LOGFILE"
}

get_opencode_pids() {
    pgrep -f '.opencode run' 2>/dev/null || true
}

get_pid_info() {
    local pid=$1
    local procdir="/proc/${pid}"
    
    if [ -d "$procdir" ]; then
        local stat state ppid
        stat=$(cat "${procdir}/stat" 2>/dev/null || echo "unknown")
        state=$(echo "$stat" | awk '{print $3}' | tr -d '()')
        ppid=$(echo "$stat" | awk '{print $4}')
        local vmrss=$(grep VmRSS "${procdir}/status" 2>/dev/null | awk '{print $2}' || echo "?")
        local vmsize=$(grep VmSize "${procdir}/status" 2>/dev/null | awk '{print $2}' || echo "?")
        local threads=$(grep Threads "${procdir}/status" 2>/dev/null | awk '{print $2}' || echo "?")
        local sigpend=$(grep SigPnd "${procdir}/status" 2>/dev/null | awk '{print $2}' || echo "?")
        local sigblk=$(grep SigBlk "${procdir}/status" 2>/dev/null | awk '{print $2}' || echo "?")
        local sigign=$(grep SigIgn "${procdir}/status" 2>/dev/null | awk '{print $2}' || echo "?")
        local oom_score=$(cat "${procdir}/oom_score" 2>/dev/null || echo "?")
        local oom_adj=$(cat "${procdir}/oom_score_adj" 2>/dev/null || echo "?")
        
        echo "state=${state} ppid=${ppid} vmrss=${vmrss}kB vmsize=${vmsize}kB threads=${threads} sigpend=${sigpend} oom_score=${oom_score} oom_adj=${oom_adj}"
    else
        echo "GONE"
    fi
}

log "=== opencode-monitor started ==="
log "Polling every ${POLL_INTERVAL}s for opencode processes"

while true; do
    ITERATION=$((ITERATION + 1))
    CURRENT_PIDS=$(get_opencode_pids)
    
    if [ -z "$CURRENT_PIDS" ]; then
        if [ $((ITERATION % 60)) -eq 0 ]; then
            log "poll #${ITERATION}: no opencode processes"
        fi
        sleep "$POLL_INTERVAL"
        continue
    fi
    
    for pid in $CURRENT_PIDS; do
        if ! echo " $KNOWN_PIDS " | grep -q " $pid "; then
            info=$(get_pid_info "$pid")
            cmdline=$(tr '\0' ' ' < "/proc/${pid}/cmdline" 2>/dev/null | head -c 200 || echo "?")
            log "NEW pid=${pid} ${info} cmd=${cmdline}"
            KNOWN_PIDS=" $KNOWN_PIDS $pid "
        fi
    done
    
    # Check known processes for state changes or exits
    NEW_KNOWN=""
    for pid in $KNOWN_PIDS; do
        [ -z "$pid" ] && continue
        info=$(get_pid_info "$pid")
        if [ "$info" = "GONE" ]; then
            # Check if there's any exit info in /proc (won't be, but check dmesg)
            log "EXIT pid=${pid} -- process gone"
            # Check recent dmesg for this PID
            dmesg_out=$(dmesg --time-format iso 2>/dev/null | grep -i "pid=${pid}\|${pid}" | tail -5 || true)
            if [ -n "$dmesg_out" ]; then
                log "DMESG for pid=${pid}: ${dmesg_out}"
            fi
        else
            # Check if oom_score is high (>100 means likely OOM victim)
            oom=$(echo "$info" | grep -o 'oom_score=[0-9]*' | cut -d= -f2)
            if [ -n "$oom" ] && [ "$oom" -gt 100 ]; then
                log "WARN pid=${pid} oom_score=${oom} ${info}"
            fi
            NEW_KNOWN=" $NEW_KNOWN $pid "
        fi
    done
    KNOWN_PIDS="$NEW_KNOWN"
    
    sleep "$POLL_INTERVAL"
done
