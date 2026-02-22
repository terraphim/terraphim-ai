#!/bin/bash

# Claude Code Session 1: Fast & Expensive Routing
# Uses premium Claude Sonnet 4.5 for high-performance, critical tasks

SESSION_NAME="claude-fast-expensive"
PROXY_PORT=3456

echo "🚀 Starting Claude Code Session: Fast & Expensive Routing"
echo "Session Name: $SESSION_NAME"
echo "Proxy Port: $PROXY_PORT"
echo "Expected Model: anthropic/claude-sonnet-4.5 (premium)"
echo ""

# Kill existing session if it exists
tmux kill-session -t $SESSION_NAME 2>/dev/null || true

# Create new tmux session
tmux new-session -d -s $SESSION_NAME

# Split window: top for proxy, bottom for Claude Code
tmux split-window -h

# Start proxy in left pane
tmux send-keys -t $SESSION_NAME:0.0 "cd /home/alex/projects/terraphim-llm-proxy" Enter
tmux send-keys -t $SESSION_NAME:0.0 "echo '🔧 Starting LLM Proxy for Fast & Expensive routing...'" Enter
tmux send-keys -t $SESSION_NAME:0.0 "./start_proxy_verbose_debug.sh" Enter

# Wait for proxy to start
sleep 3

# Configure Claude Code in right pane
tmux send-keys -t $SESSION_NAME:0.1 "cd /home/alex/projects/terraphim-llm-proxy" Enter
tmux send-keys -t $SESSION_NAME:0.1 "echo '🤖 Configuring Claude Code for Fast & Expensive routing...'" Enter
tmux send-keys -t $SESSION_NAME:0.1 "export ANTHROPIC_API_URL=http://localhost:$PROXY_PORT/v1" Enter
tmux send-keys -t $SESSION_NAME:0.1 "export ANTHROPIC_API_KEY=dummy-key-proxy-will-handle" Enter
tmux send-keys -t $SESSION_NAME:0.1 "echo 'Claude Code configured to use proxy on port $PROXY_PORT'" Enter

# Start Claude Code
tmux send-keys -t $SESSION_NAME:0.1 "echo '🎯 Starting Claude Code...'" Enter
tmux send-keys -t $SESSION_NAME:0.1 "claude" Enter

# Attach to the session
echo "✅ Fast & Expensive routing session started!"
echo "📝 Test prompts for this scenario:"
echo "   - 'urgent critical production issue needs immediate resolution'"
echo "   - 'enterprise grade application requires maximum performance'"
echo "   - 'realtime decision making needed urgently'"
echo ""
echo "🖥️  Attaching to tmux session: $SESSION_NAME"
echo "   Left pane: LLM Proxy logs"
echo "   Right pane: Claude Code"
echo ""
tmux attach-session -t $SESSION_NAME