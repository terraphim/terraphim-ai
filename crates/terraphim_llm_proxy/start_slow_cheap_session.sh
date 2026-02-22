#!/bin/bash

# Claude Code Session 4: Slow & Cheap Routing
# Uses DeepSeek Chat for cost-effective background processing

SESSION_NAME="claude-slow-cheap"
PROXY_PORT=3459

echo "💰 Starting Claude Code Session: Slow & Cheap Routing"
echo "Session Name: $SESSION_NAME"
echo "Proxy Port: $PROXY_PORT"
echo "Expected Model: deepseek/deepseek-chat (budget)"
echo ""

# Create a temporary config for this session with different port
CONFIG_FILE="config_slow_cheap.toml"
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
tmux send-keys -t $SESSION_NAME:0.0 "echo '🔧 Starting LLM Proxy for Slow & Cheap routing...'" Enter
tmux send-keys -t $SESSION_NAME:0.0 "RUST_LOG=info cargo run --release --config $CONFIG_FILE" Enter

# Wait for proxy to start
sleep 3

# Configure Claude Code in right pane
tmux send-keys -t $SESSION_NAME:0.1 "cd /home/alex/projects/terraphim-llm-proxy" Enter
tmux send-keys -t $SESSION_NAME:0.1 "echo '🤖 Configuring Claude Code for Slow & Cheap routing...'" Enter
tmux send-keys -t $SESSION_NAME:0.1 "export ANTHROPIC_API_URL=http://localhost:$PROXY_PORT/v1" Enter
tmux send-keys -t $SESSION_NAME:0.1 "export ANTHROPIC_API_KEY=dummy-key-proxy-will-handle" Enter
tmux send-keys -t $SESSION_NAME:0.1 "echo 'Claude Code configured to use proxy on port $PROXY_PORT'" Enter

# Start Claude Code
tmux send-keys -t $SESSION_NAME:0.1 "echo '🎯 Starting Claude Code...'" Enter
tmux send-keys -t $SESSION_NAME:0.1 "claude" Enter

# Attach to the session
echo "✅ Slow & Cheap routing session started!"
echo "📝 Test prompts for this scenario:"
echo "   - 'I need a cheap solution for this background task'"
echo "   - 'Can you process this slowly to save money?'"
echo "   - 'Use the most economical approach for this data processing'"
echo "   - 'Budget is more important than speed for this batch job'"
echo ""
echo "🖥️  Attaching to tmux session: $SESSION_NAME"
echo "   Left pane: LLM Proxy logs"
echo "   Right pane: Claude Code"
echo ""
tmux attach-session -t $SESSION_NAME