#!/bin/bash
#
# ADF Infrastructure Orchestrator - Health Monitor & Auto-Recovery
# Prevents Gitea infrastructure crashes caused by docker system prune
#
# Installation:
#   sudo cp adf-orchestrator.sh /usr/local/bin/
#   sudo chmod +x /usr/local/bin/adf-orchestrator.sh
#   sudo systemctl enable adf-orchestrator.service
#

set -euo pipefail

# Configuration
STACK_DIR="/home/alex/gitea-stack"
COMPOSE_FILE="docker-compose.yml"
LOG_FILE="/var/log/adf-orchestrator.log"
ALERT_EMAIL="admin@terraphim.cloud"
HEALTH_CHECK_INTERVAL=30
MAX_RESTART_ATTEMPTS=3
RESTART_WINDOW=300  # 5 minutes

# Services to monitor (in dependency order)
SERVICES=("db" "master" "volume" "filer" "s3" "prometheus" "server")

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Logging function
log() {
    local level="$1"
    shift
    local message="$*"
    local timestamp=$(date '+%Y-%m-%d %H:%M:%S')
    echo -e "${timestamp} [${level}] ${message}" | tee -a "$LOG_FILE"
}

# Check if running as root for systemctl access
check_root() {
    if [[ $EUID -ne 0 ]]; then
        log "WARN" "Not running as root. Some features may be limited."
    fi
}

# Get container status
get_container_status() {
    local service="$1"
    cd "$STACK_DIR"
    docker compose ps -q "$service" 2>/dev/null | xargs -I {} docker inspect -f '{{.State.Status}}' {} 2>/dev/null || echo "not_found"
}

# Check if container is healthy
check_health() {
    local service="$1"
    local status=$(get_container_status "$service")

    case "$status" in
        "running")
            # Check health status if available
            local health=$(cd "$STACK_DIR" && docker compose ps -q "$service" 2>/dev/null | xargs -I {} docker inspect -f '{{.State.Health.Status}}' {} 2>/dev/null || echo "none")
            if [[ "$health" == "healthy" ]] || [[ "$health" == "none" ]]; then
                return 0
            else
                log "WARN" "Service $service is running but health check status: $health"
                return 1
            fi
            ;;
        "exited"|"dead"|"not_found")
            return 1
            ;;
        *)
            log "WARN" "Unknown status '$status' for service $service"
            return 1
            ;;
    esac
}

# Restart a service
restart_service() {
    local service="$1"
    log "INFO" "${YELLOW}Restarting service: $service${NC}"

    cd "$STACK_DIR"

    # Stop if running
    docker compose stop "$service" 2>/dev/null || true
    sleep 2

    # Remove container to ensure clean state
    docker compose rm -f "$service" 2>/dev/null || true
    sleep 1

    # Start service
    if docker compose up -d "$service"; then
        log "INFO" "${GREEN}Successfully restarted $service${NC}"

        # Wait for health check
        sleep 10
        local attempts=0
        while [[ $attempts -lt 6 ]]; do
            if check_health "$service"; then
                log "INFO" "${GREEN}Service $service is healthy${NC}"
                return 0
            fi
            attempts=$((attempts + 1))
            log "INFO" "Waiting for $service to become healthy (attempt $attempts/6)..."
            sleep 5
        done

        log "ERROR" "${RED}Service $service failed health check after restart${NC}"
        return 1
    else
        log "ERROR" "${RED}Failed to restart $service${NC}"
        return 1
    fi
}

# Monitor all services
monitor_services() {
    local failed_services=()

    for service in "${SERVICES[@]}"; do
        if ! check_health "$service"; then
            failed_services+=("$service")
        fi
    done

    if [[ ${#failed_services[@]} -eq 0 ]]; then
        return 0
    fi

    log "WARN" "${YELLOW}Detected ${#failed_services[@]} failed service(s): ${failed_services[*]}${NC}"

    # Restart failed services in dependency order
    for service in "${SERVICES[@]}"; do
        if [[ " ${failed_services[*]} " =~ " ${service} " ]]; then
            restart_service "$service" || true
        fi
    done
}

# Protect against docker system prune by ensuring all containers are running
protect_against_prune() {
    # Ensure stack directory exists
    if [[ ! -d "$STACK_DIR" ]]; then
        log "ERROR" "${RED}Stack directory $STACK_DIR does not exist!${NC}"
        return 1
    fi

    local not_running=()

    for service in "${SERVICES[@]}"; do
        local status=$(cd "$STACK_DIR" && docker compose ps -q "$service" 2>/dev/null | xargs -I {} docker inspect -f '{{.State.Status}}' {} 2>/dev/null || echo "not_found")
        if [[ "$status" != "running" ]]; then
            not_running+=("$service")
        fi
    done

    if [[ ${#not_running[@]} -gt 0 ]]; then
        log "WARN" "${YELLOW}Found ${#not_running[@]} stopped container(s) - possible docker prune impact: ${not_running[*]}${NC}"

        # Recreate and restart all services to ensure consistency
        log "INFO" "Performing full stack restart to recover from potential prune..."
        cd "$STACK_DIR"
        docker compose up -d --remove-orphans

        # Wait for all services
        sleep 30
        monitor_services
    fi
}

# Check disk space
check_disk_space() {
    local usage=$(df -h / | awk 'NR==2 {print $5}' | tr -d '%')
    if [[ $usage -gt 90 ]]; then
        log "ERROR" "${RED}CRITICAL: Disk usage is at ${usage}%. Clean up required!${NC}"
        # Alert via email if configured
        if command -v mail &> /dev/null && [[ -n "$ALERT_EMAIL" ]]; then
            echo "Disk usage critical: ${usage}% on $(hostname)" | mail -s "ADF Alert: Low Disk Space" "$ALERT_EMAIL" || true
        fi
        return 1
    elif [[ $usage -gt 80 ]]; then
        log "WARN" "${YELLOW}WARNING: Disk usage is at ${usage}%. Monitor closely.${NC}"
    fi
    return 0
}

# Check memory usage
check_memory() {
    local usage=$(free | grep Mem | awk '{printf("%.0f", $3/$2 * 100.0)}')
    if [[ $usage -gt 95 ]]; then
        log "ERROR" "${RED}CRITICAL: Memory usage is at ${usage}%. Risk of OOM kills!${NC}"
        return 1
    elif [[ $usage -gt 85 ]]; then
        log "WARN" "${YELLOW}WARNING: Memory usage is at ${usage}%.${NC}"
    fi
    return 0
}

# Main monitoring loop
main_loop() {
    log "INFO" "${GREEN}=== ADF Orchestrator Started ===${NC}"
    log "INFO" "Monitoring services: ${SERVICES[*]}"
    log "INFO" "Health check interval: ${HEALTH_CHECK_INTERVAL}s"

    while true; do
        # System health checks
        check_disk_space || true
        check_memory || true

        # Service health checks
        protect_against_prune
        monitor_services

        # Wait before next check
        sleep "$HEALTH_CHECK_INTERVAL"
    done
}

# Handle signals gracefully
cleanup() {
    log "INFO" "${YELLOW}ADF Orchestrator shutting down...${NC}"
    exit 0
}

trap cleanup SIGTERM SIGINT

# Ensure log directory exists
mkdir -p "$(dirname "$LOG_FILE")" || true

# Run main loop
check_root
main_loop
