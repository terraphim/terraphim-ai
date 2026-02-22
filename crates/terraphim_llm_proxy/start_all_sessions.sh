#!/bin/bash

# Master script to start all 4 Claude Code routing scenario sessions
# Each session demonstrates a different routing scenario with the LLM proxy

echo "🎯 Starting all 4 Claude Code routing scenario sessions..."
echo "This will create 4 tmux sessions, each testing a different routing scenario."
echo ""

# Make all session scripts executable
chmod +x start_fast_expensive_session.sh
chmod +x start_intelligent_session.sh
chmod +x start_balanced_session.sh
chmod +x start_slow_cheap_session.sh

echo "📋 Available sessions:"
echo "1. 🚀 Fast & Expensive - Premium Claude Sonnet 4.5 for critical tasks"
echo "2. 🧠 Intelligent - DeepSeek V3.1 Terminus for reasoning tasks"
echo "3. ⚖️  Balanced - Claude 3.5 Sonnet for regular tasks"
echo "4. 💰 Slow & Cheap - DeepSeek Chat for budget tasks"
echo ""

# Ask user which sessions to start
read -p "Start all 4 sessions? (y/n): " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Session startup cancelled."
    exit 0
fi

echo ""
echo "🚀 Starting sessions one by one..."
echo ""

# Start each session in background, giving them time to initialize
echo "1️⃣  Starting Fast & Expensive session..."
./start_fast_expensive_session.sh &
sleep 2

echo "2️⃣  Starting Intelligent session..."
./start_intelligent_session.sh &
sleep 2

echo "3️⃣  Starting Balanced session..."
./start_balanced_session.sh &
sleep 2

echo "4️⃣  Starting Slow & Cheap session..."
./start_slow_cheap_session.sh &
sleep 2

echo ""
echo "✅ All sessions started!"
echo ""
echo "📊 Session Summary:"
echo "   tmux attach-session -t claude-fast-expensive  🚀 Fast & Expensive"
echo "   tmux attach-session -t claude-intelligent      🧠 Intelligent"
echo "   tmux attach-session -t claude-balanced        ⚖️  Balanced"
echo "   tmux attach-session -t claude-slow-cheap       💰 Slow & Cheap"
echo ""
echo "🔍 To see all running sessions: tmux list-sessions"
echo "🛑 To kill all sessions: tmux kill-server"
echo ""
echo "🎯 Each session is configured to test different routing keywords."
echo "    Try the suggested prompts in each session to verify routing works!"
echo ""

# Show current tmux sessions
echo "📋 Current tmux sessions:"
tmux list-sessions 2>/dev/null || echo "No tmux sessions found"