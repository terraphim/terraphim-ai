#!/bin/bash

# Claude Code Session 2: Intelligent Routing
# Uses DeepSeek V3.1 Terminus for complex reasoning and planning tasks

SESSION_NAME="claude-intelligent"
PROXY_PORT=3457

echo "🧠 Starting Claude Code Session: Intelligent Routing"
echo "Session Name: $SESSION_NAME"
echo "Proxy Port: $PROXY_PORT"
echo "Expected Model: deepseek/deepseek-v3.1-terminus (reasoning)"
echo ""

# Create a temporary config for this session with different port
CONFIG_FILE="config_intelligent.toml"
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
tmux send-keys -t $SESSION_NAME:0.0 "echo '🔧 Starting LLM Proxy for Intelligent routing...'" Enter
tmux send-keys -t $SESSION_NAME:0.0 "RUST_LOG=trace cargo run --release --config $CONFIG_FILE" Enter

# Wait for proxy to start
sleep 3

# Configure Claude Code in right pane
tmux send-keys -t $SESSION_NAME:0.1 "cd /home/alex/projects/terraphim-llm-proxy" Enter
tmux send-keys -t $SESSION_NAME:0.1 "echo '🤖 Configuring Claude Code for Intelligent routing...'" Enter
tmux send-keys -t $SESSION_NAME:0.1 "export ANTHROPIC_API_URL=http://localhost:$PROXY_PORT/v1" Enter
tmux send-keys -t $SESSION_NAME:0.1 "export ANTHROPIC_API_KEY=dummy-key-proxy-will-handle" Enter
tmux send-keys -t $SESSION_NAME:0.1 "echo 'Claude Code configured to use proxy on port $PROXY_PORT'" Enter

# Start Claude Code
tmux send-keys -t $SESSION_NAME:0.1 "echo '🎯 Starting Claude Code...'" Enter
tmux send-keys -t $SESSION_NAME:0.1 "claude" Enter

# Attach to the session
echo "✅ Intelligent routing session started!"
echo "📝 Test prompts for this scenario:"
echo "   - 'I need to think through this architecture design step by step'"
echo "   - 'Please plan out this implementation systematically'"
echo "   - 'Let me analyze this problem with logical reasoning'"
echo "   - 'Work through this design thinking exercise carefully'"
echo ""
echo "🖥️  Attaching to tmux session: $SESSION_NAME"
echo "   Left pane: LLM Proxy logs"
echo "   Right pane: Claude Code"
echo ""
tmux attach-session -t $SESSION_NAME