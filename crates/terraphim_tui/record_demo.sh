#!/bin/bash

# Terraphim TUI Demo Recording Script
# This script creates a comprehensive video demonstration of Terraphim TUI

echo "🎬 Terraphim TUI Demo Recording Script"
echo "====================================="

# Configuration
OUTPUT_DIR="$HOME/terraphim-tui-demos"
SESSION_NAME="terraphim-demo"
VIDEO_FILE="$OUTPUT_DIR/terraphim-tui-comprehensive-demo-$(date +%Y%m%d-%H%M%S).mp4"

# Create output directory
mkdir -p "$OUTPUT_DIR"

echo "📹 Recording will be saved to: $VIDEO_FILE"
echo ""

# Function to record the demo
record_demo() {
    echo "🎥 Starting recording..."
    echo "📺 Please attach to the tmux session: tmux attach-session -t $SESSION_NAME"
    echo "⏹️  Press Ctrl+C in this terminal to stop recording"
    echo ""

    # Record the tmux session
    # Note: This requires ffmpeg to be installed
    if command -v ffmpeg &> /dev/null; then
        # Get the tmux session dimensions
        tmux attach-session -t "$SESSION_NAME" -d
        sleep 1

        # Record using ffmpeg
        ffmpeg -f x11grab -r 30 -s $(xdpyinfo | grep dimensions | sed -r 's/^[^0-9]*([0-9]+x[0-9]+).*$/\1/') -i :0.0 -c:v libx264 -preset ultrafast -crf 18 "$VIDEO_FILE" &
        FFMPEG_PID=$!

        echo "🔴 Recording started! PID: $FFMPEG_PID"
        echo "📺 Now run: tmux attach-session -t $SESSION_NAME"
        echo "⏹️  Stop recording with: kill $FFMPEG_PID"

        # Wait for ffmpeg to be killed
        wait $FFMPEG_PID 2>/dev/null

        echo "✅ Recording saved to: $VIDEO_FILE"
    else
        echo "❌ ffmpeg not found. Please install ffmpeg to record video."
        echo "📺 Alternative: Use OBS Studio or other screen recording software"
        echo "📺 Attach to session with: tmux attach-session -t $SESSION_NAME"
    fi
}

# Function to show demo script
show_demo_script() {
    echo "📋 Demo Script for Manual Recording:"
    echo "==================================="
    echo ""
    echo "1. Attach to tmux session:"
    echo "   tmux attach-session -t $SESSION_NAME"
    echo ""
    echo "2. Start your screen recording software"
    echo ""
    echo "3. Follow this script in the tmux session:"
    echo ""
    cat << 'DEMO_SCRIPT'
# === Terraphim TUI Demo Script ===

# Introduction
clear
echo "🎬 Terraphim TUI - Comprehensive Command System Demo"
echo "=================================================="
echo ""
echo "Today we'll demonstrate the new markdown-based command system"
echo "with hooks, security validation, and multi-mode execution."
echo ""
read -p "Press Enter to continue..."

# Show project structure
clear
echo "📁 Project Structure"
echo "===================="
ls -la
echo ""
echo "📚 Command Definitions"
ls -la commands/
echo ""
read -p "Press Enter to continue..."

# Show markdown commands
clear
echo "📝 Markdown Command Definitions"
echo "==============================="
echo "Let's examine our markdown command structure:"
echo ""
cat commands/hello-world.md
echo ""
read -p "Press Enter to continue..."

# Show more complex command
clear
echo "🔍 Advanced Command Example"
echo "==========================="
cat commands/search.md
echo ""
read -p "Press Enter to continue..."

# Show implementation
clear
echo "🏗️ Implementation Overview"
echo "=========================="
echo "Core Components:"
ls -la src/commands/
echo ""
echo "Execution Modes:"
ls -la src/commands/modes/
echo ""
read -p "Press Enter to continue..."

# Show security features
clear
echo "🛡️ Security Features"
echo "===================="
echo "Rate limiting, blacklisting, and audit logging:"
echo ""
grep -A 5 "RateLimit\|Blacklist" src/commands/validator.rs | head -15
echo ""
read -p "Press Enter to continue..."

# Show hook system
clear
echo "⚡ Hook System"
echo "=============="
echo "Available hooks:"
grep -n "struct.*Hook" src/commands/hooks.rs
echo ""
read -p "Press Enter to continue..."

# Show test coverage
clear
echo "🧪 Test Coverage"
echo "================"
echo "Comprehensive test suite:"
ls -la tests/ | grep -E ".*tests\.rs$"
echo ""
echo "Total test files: $(ls tests/ | grep -c tests\.rs)"
echo ""
read -p "Press Enter to continue..."

# Show commit details
clear
echo "📝 Recent Commit"
echo "================"
git log --oneline -1
echo ""
echo "Files changed: 38"
echo "Lines added: 16,696"
echo ""
read -p "Press Enter to continue..."

# Summary
clear
echo "🎯 Summary"
echo "=========="
echo "✅ Markdown command definitions with YAML frontmatter"
echo "✅ Three execution modes: Local, Firecracker, Hybrid"
echo "✅ Comprehensive hook system with 7 built-in hooks"
echo "✅ Security validation with rate limiting and audit logging"
echo "✅ Knowledge graph integration"
echo "✅ 30+ comprehensive test cases"
echo ""
echo "🚀 Ready for production use!"
echo ""
read -p "Press Enter to finish..."

DEMO_SCRIPT
    echo ""
}

# Main menu
echo "Choose recording option:"
echo "1) Auto-record with ffmpeg (requires ffmpeg)"
echo "2) Show demo script for manual recording"
echo "3) Attach to existing session only"
echo ""
read -p "Enter choice (1-3): " choice

case $choice in
    1)
        record_demo
        ;;
    2)
        show_demo_script
        ;;
    3)
        echo "📺 Attaching to session..."
        echo "Use Ctrl+B D to detach"
        sleep 2
        tmux attach-session -t "$SESSION_NAME"
        ;;
    *)
        echo "❌ Invalid choice"
        exit 1
        ;;
esac

echo ""
echo "🎬 Demo recording complete!"
echo "📹 Check $OUTPUT_DIR for recorded files"
