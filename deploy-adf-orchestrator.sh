#!/bin/bash
#
# Deploy ADF Orchestrator systemd service to bigbox.
# Prerequisites: ADF binary already at /usr/local/bin/adf on bigbox.
#
set -euo pipefail

echo "=== Deploying ADF Orchestrator service to bigbox ==="

# Copy systemd unit file
echo "Installing systemd service..."
scp adf-orchestrator.service bigbox:/tmp/
ssh bigbox "sudo cp /tmp/adf-orchestrator.service /etc/systemd/system/ && rm /tmp/adf-orchestrator.service"

# Reload systemd, enable on boot, (re)start
echo "Enabling and starting service..."
ssh bigbox "sudo systemctl daemon-reload && sudo systemctl enable adf-orchestrator.service"

echo ""
echo "Service installed and enabled."
echo ""
echo "To start:   ssh bigbox 'sudo systemctl start adf-orchestrator'"
echo "To stop:    ssh bigbox 'sudo systemctl stop adf-orchestrator'"
echo "To restart: ssh bigbox 'sudo systemctl restart adf-orchestrator'"
echo "Logs:       ssh bigbox 'journalctl -u adf -f'"
echo ""
echo "NOTE: not starting automatically -- wait for current ADF run to finish first."
