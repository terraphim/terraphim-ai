#!/bin/bash

# Claude Code Session 3: Balanced Routing
# Uses Claude 3.5 Sonnet for optimal cost/performance balance

SESSION_NAME="claude-balanced"
PROXY_PORT=3458

echo "⚖️  Starting Claude Code Session: Balanced Routing"
echo "Session Name: $SESSION_NAME"
echo "Proxy Port: $PROXY_PORT"
echo "Expected Model: anthropic/claude-3.5-sonnet (balanced)"
echo ""

# Create a temporary config for this session with different port
CONFIG_FILE="config_balanced.toml"
cp config.toml $CONFIG_FILE
sed -i "s/port = 3456/port = $PROXY_PORT/" $CONFIG_FILE

# Kill existing session if it exists
tmux kill-session -t $SESSION_NAME 2>/dev/null || true

# Create new tmux session
tmux new-session -d -s $SESSION_NAME

# Split window: top for proxy, bottom for Claude Code
tmux split-window -h

# Start proxy in left pane
tmux send-keys -t $SESSION_NAME:0.0 "cd /home/alex/projects/terraphim-llm-proxy" Enter
tmux send-keys -t $SESSION_NAME:0.0 "echo '🔧 Starting LLM Proxy for Balanced routing...'" Enter
tmux send-keys -t $SESSION_NAME:0.0 "RUST_LOG=debug cargo run --release --config $CONFIG_FILE" Enter

# Wait for proxy to start
sleep 3

# Configure Claude Code in right pane
tmux send-keys -t $SESSION_NAME:0.1 "cd /home/alex/projects/terraphim-llm-proxy" Enter
tmux send-keys -t $SESSION_NAME:0.1 "echo '🤖 Configuring Claude Code for Balanced routing...'" Enter
tmux send-keys -t $SESSION_NAME:0.1 "export ANTHROPIC_API_URL=http://localhost:$PROXY_PORT/v1" Enter
tmux send-keys -t $SESSION_NAME:0.1 "export ANTHROPIC_API_KEY=dummy-key-proxy-will-handle" Enter
tmux send-keys -t $SESSION_NAME:0.1 "echo 'Claude Code configured to use proxy on port $PROXY_PORT'" Enter

# Start Claude Code
tmux send-keys -t $SESSION_NAME:0.1 "echo '🎯 Starting Claude Code...'" Enter
tmux send-keys -t $SESSION_NAME:0.1 "claude" Enter

# Attach to the session
echo "✅ Balanced routing session started!"
echo "📝 Test prompts for this scenario:"
echo "   - 'Help me understand this code'"
echo "   - 'What is a standard approach to debugging this issue?'"
echo "   - 'Explain this concept in a practical way'"
echo "   - 'What is the regular solution to this problem?'"
echo ""
echo "🖥️  Attaching to tmux session: $SESSION_NAME"
echo "   Left pane: LLM Proxy logs"
echo "   Right pane: Claude Code"
echo ""
tmux attach-session -t $SESSION_NAME