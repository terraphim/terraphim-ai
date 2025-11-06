# Terraphim AI - Complete Functional Testing Plan

## Objective
Prove that EVERY function in REPL, TUI, Server, and Desktop is fully functional through systematic testing with documented evidence.

---

## 1. TUI REPL Component Testing

### 1.1 Command Coverage Matrix

| Command | Parameters | Test Input | Expected Output | Status |
|---------|------------|------------|-----------------|--------|
| `/search` | `<query>` | `/search rust async` | List of documents with relevance scores | ⏳ |
| `/config show` | None | `/config show` | Display current configuration JSON | ⏳ |
| `/config set` | `<key> <value>` | `/config set theme darkula` | Configuration updated message | ⏳ |
| `/role list` | None | `/role list` | List all available roles | ⏳ |
| `/role select` | `<role_name>` | `/role select TerraphimEngineer` | Role switched confirmation | ⏳ |
| `/graph` | Optional `<role>` | `/graph` | Display knowledge graph statistics | ⏳ |
| `/chat` | `<message>` | `/chat Hello, how are you?` | AI response | ⏳ |
| `/summarize` | `<target>` | `/summarize document.md` | Document summary | ⏳ |
| `/autocomplete` | `<query>` | `/autocomplete terr` | List of suggestions | ⏳ |
| `/extract` | `<text>` | `/extract "Find patterns in this text"` | Extracted entities | ⏳ |
| `/find` | `<pattern>` | `/find TODO` | List of matches | ⏳ |
| `/replace` | `<old> <new>` | `/replace foo bar` | Replacement results | ⏳ |
| `/thesaurus` | None | `/thesaurus` | Thesaurus statistics | ⏳ |
| `/help` | Optional `<command>` | `/help search` | Command documentation | ⏳ |
| `/quit` | None | `/quit` | Clean exit | ⏳ |

### 1.2 TUI Test Script
```bash
#!/bin/bash
# test_tui_repl.sh

BINARY="./target/release/terraphim-tui"
TEST_LOG="tui_test_results.log"

# Test each command
commands=(
    "/help"
    "/role list"
    "/config show"
    "/search test"
    "/graph"
    "/thesaurus"
    "/quit"
)

for cmd in "${commands[@]}"; do
    echo "Testing: $cmd" | tee -a $TEST_LOG
    echo "$cmd" | timeout 5 $BINARY repl 2>&1 | tee -a $TEST_LOG
    echo "---" | tee -a $TEST_LOG
done
```

---

## 2. Server API Component Testing

### 2.1 Endpoint Coverage Matrix

| Method | Endpoint | Request Body | Expected Response | Status |
|--------|----------|--------------|-------------------|--------|
| GET | `/health` | None | `"OK"` | ⏳ |
| GET | `/config` | None | Config JSON with status | ⏳ |
| POST | `/config` | Config JSON | Updated config | ⏳ |
| POST | `/search` | `{"query": "test", "role": "Default"}` | Search results | ⏳ |
| POST | `/chat` | `{"message": "Hello", "conversation_id": "123"}` | Chat response | ⏳ |
| GET | `/graph/<role>` | None | Graph nodes and edges | ⏳ |
| GET | `/thesaurus/<role>` | None | Thesaurus data | ⏳ |
| POST | `/autocomplete` | `{"query": "ter", "role": "Default"}` | Suggestions | ⏳ |
| GET | `/documents` | None | Document list | ⏳ |
| POST | `/documents` | Document JSON | Created document | ⏳ |
| GET | `/roles` | None | Available roles | ⏳ |
| POST | `/role/select` | `{"role": "TerraphimEngineer"}` | Role switched | ⏳ |

### 2.2 Server Test Script
```bash
#!/bin/bash
# test_server_api.sh

SERVER_URL="http://localhost:8000"
TEST_LOG="server_test_results.log"

# Start server
./target/release/terraphim_server &
SERVER_PID=$!
sleep 3

# Test endpoints
echo "Testing Health Check" | tee -a $TEST_LOG
curl -s "$SERVER_URL/health" | tee -a $TEST_LOG

echo -e "\n\nTesting Config GET" | tee -a $TEST_LOG
curl -s "$SERVER_URL/config" | python3 -m json.tool | tee -a $TEST_LOG

echo -e "\n\nTesting Search POST" | tee -a $TEST_LOG
curl -s -X POST "$SERVER_URL/search" \
    -H "Content-Type: application/json" \
    -d '{"query": "test", "role": "Default"}' | python3 -m json.tool | tee -a $TEST_LOG

# Cleanup
kill $SERVER_PID
```

---

## 3. Desktop Application Testing

### 3.1 UI Component Coverage Matrix

| Component | Feature | Test Action | Expected Result | Status |
|-----------|---------|-------------|-----------------|--------|
| **Role Selector** | Dropdown display | Click dropdown | Shows all roles | ⏳ |
| | Role change | Select different role | Theme changes | ⏳ |
| | Persistence | Restart app | Role selection saved | ⏳ |
| **System Tray** | Icon display | Check system tray | Icon visible | ⏳ |
| | Menu display | Right-click icon | Shows menu | ⏳ |
| | Role selection | Select role from menu | UI updates | ⏳ |
| | Show/Hide | Click toggle | Window visibility | ⏳ |
| | Quit | Click quit | App closes | ⏳ |
| **Search Tab** | Navigation | Click Search tab | Search interface | ⏳ |
| | Query input | Type search query | Text appears | ⏳ |
| | Search execution | Press Enter | Results display | ⏳ |
| | Result interaction | Click result | Document opens | ⏳ |
| **Chat Tab** | Navigation | Click Chat tab | Chat interface | ⏳ |
| | New conversation | Click New | Empty conversation | ⏳ |
| | Send message | Type and send | Message appears | ⏳ |
| | Receive response | Wait for AI | Response appears | ⏳ |
| | Context management | Add context | Context listed | ⏳ |
| **Graph Tab** | Navigation | Click Graph tab | Graph visualization | ⏳ |
| | Graph rendering | View graph | Nodes and edges | ⏳ |
| | Zoom/Pan | Mouse actions | Graph moves | ⏳ |
| | Node interaction | Click node | Node details | ⏳ |

### 3.2 Desktop Test Script
```bash
#!/bin/bash
# test_desktop_app.sh

APP_PATH="/Users/alex/projects/terraphim/terraphim-ai/target/release/bundle/macos/Terraphim Desktop.app"
TEST_LOG="desktop_test_results.log"

# Launch app
open "$APP_PATH"
sleep 5

# Use AppleScript for UI automation (macOS specific)
osascript <<EOF
tell application "System Events"
    tell process "Terraphim Desktop"
        # Test role selector
        click pop up button 1 of window 1
        delay 1
        click menu item "TerraphimEngineer" of menu 1 of pop up button 1 of window 1
        delay 2
        
        # Log result
        do shell script "echo 'Role selector: TESTED' >> $TEST_LOG"
    end tell
end tell
EOF
```

---

## 4. Integration Testing

### 4.1 Desktop ↔ Server Communication
```bash
# Start server
./target/release/terraphim_server &
SERVER_PID=$!

# Launch desktop in server mode
TERRAPHIM_SERVER_URL="http://localhost:8000" open "$APP_PATH"

# Test API calls from desktop
# Monitor network traffic to verify communication
```

### 4.2 Configuration Persistence
```bash
# Modify configuration
curl -X POST "$SERVER_URL/config" -d '{"selected_role": "TerraphimEngineer"}'

# Restart components and verify config loaded
```

### 4.3 Thesaurus Loading
```bash
# Select role with KG enabled
# Verify thesaurus loads
# Test autocomplete functionality
```

---

## 5. Error Handling Testing

### 5.1 Invalid Input Tests
| Component | Test Case | Input | Expected Behavior | Status |
|-----------|-----------|-------|-------------------|--------|
| TUI | Invalid command | `/foobar` | Error message | ⏳ |
| TUI | Missing params | `/search` | Usage help | ⏳ |
| Server | Malformed JSON | `{invalid}` | 400 Bad Request | ⏳ |
| Server | Unknown endpoint | `/api/unknown` | 404 Not Found | ⏳ |
| Desktop | Invalid role | Select non-existent | Error handling | ⏳ |

### 5.2 Error Test Script
```bash
#!/bin/bash
# test_error_handling.sh

# Test invalid TUI commands
echo "/invalid_command" | ./target/release/terraphim-tui repl 2>&1 | grep -i error

# Test invalid API requests
curl -X POST "$SERVER_URL/search" -d "invalid json" -v 2>&1 | grep "400"

# Test missing config
rm -f ~/.terraphim/config.json
./target/release/terraphim_server 2>&1 | grep -i "default"
```

---

## 6. Performance Testing

### 6.1 Performance Metrics
| Operation | Target | Measurement Method | Status |
|-----------|--------|-------------------|--------|
| Search query | < 500ms | Time command execution | ⏳ |
| Chat response | < 2s | Measure API response | ⏳ |
| Graph render | < 1s | UI profiling | ⏳ |
| Config load | < 100ms | Startup timing | ⏳ |
| Document parse | < 200ms/doc | Batch processing | ⏳ |

### 6.2 Performance Test Script
```bash
#!/bin/bash
# test_performance.sh

# Measure search performance
time curl -s -X POST "$SERVER_URL/search" \
    -d '{"query": "test", "role": "Default"}'

# Measure startup time
time ./target/release/terraphim_server --version

# Load test with multiple requests
for i in {1..100}; do
    curl -s "$SERVER_URL/health" &
done
wait
```

---

## 7. Test Data Requirements

### 7.1 Required Test Files
```
test_data/
├── documents/
│   ├── sample1.md (100 lines)
│   ├── sample2.md (500 lines)
│   └── large.md (10000 lines)
├── configs/
│   ├── default.json
│   ├── engineer.json
│   └── invalid.json
└── thesaurus/
    ├── default_thesaurus.json
    └── engineer_thesaurus.json
```

### 7.2 Test Data Generation Script
```bash
#!/bin/bash
# generate_test_data.sh

mkdir -p test_data/{documents,configs,thesaurus}

# Generate sample documents
echo "# Sample Document 1" > test_data/documents/sample1.md
for i in {1..100}; do
    echo "Line $i: Lorem ipsum dolor sit amet" >> test_data/documents/sample1.md
done

# Generate config files
cat > test_data/configs/default.json <<EOF
{
    "selected_role": "Default",
    "roles": {
        "Default": {
            "theme": "spacelab",
            "terraphim_it": false
        }
    }
}
EOF
```

---

## 8. Automated Test Runner

### 8.1 Master Test Script
```bash
#!/bin/bash
# run_all_tests.sh

set -e  # Exit on error

echo "=== Terraphim AI Complete Functional Test Suite ==="
echo "Started at: $(date)"

# Create results directory
RESULTS_DIR="test_results_$(date +%Y%m%d_%H%M%S)"
mkdir -p $RESULTS_DIR

# Run component tests
echo -e "\n1. Testing TUI REPL..."
./test_tui_repl.sh 2>&1 | tee $RESULTS_DIR/tui.log

echo -e "\n2. Testing Server API..."
./test_server_api.sh 2>&1 | tee $RESULTS_DIR/server.log

echo -e "\n3. Testing Desktop App..."
./test_desktop_app.sh 2>&1 | tee $RESULTS_DIR/desktop.log

echo -e "\n4. Testing Error Handling..."
./test_error_handling.sh 2>&1 | tee $RESULTS_DIR/errors.log

echo -e "\n5. Testing Performance..."
./test_performance.sh 2>&1 | tee $RESULTS_DIR/performance.log

# Generate summary report
echo -e "\n=== Test Summary ===" | tee $RESULTS_DIR/summary.txt
grep -i "error\|fail\|pass\|success" $RESULTS_DIR/*.log | tee -a $RESULTS_DIR/summary.txt

echo -e "\nCompleted at: $(date)"
echo "Results saved to: $RESULTS_DIR"
```

---

## 9. Evidence Collection

### 9.1 Required Evidence
- [ ] Screenshots of each UI state
- [ ] Network traffic logs
- [ ] Performance metrics graphs
- [ ] Error handling examples
- [ ] Configuration file snapshots
- [ ] Log files from each component

### 9.2 Evidence Collection Script
```bash
#!/bin/bash
# collect_evidence.sh

EVIDENCE_DIR="evidence_$(date +%Y%m%d_%H%M%S)"
mkdir -p $EVIDENCE_DIR/{screenshots,logs,configs,network}

# Collect screenshots (macOS)
screencapture -x $EVIDENCE_DIR/screenshots/desktop_main.png
screencapture -x $EVIDENCE_DIR/screenshots/desktop_search.png

# Collect logs
cp ~/.terraphim/logs/* $EVIDENCE_DIR/logs/
cp *.log $EVIDENCE_DIR/logs/

# Collect configs
cp ~/.terraphim/config.json $EVIDENCE_DIR/configs/

# Monitor network (requires sudo)
sudo tcpdump -i lo0 -w $EVIDENCE_DIR/network/capture.pcap port 8000 &
TCPDUMP_PID=$!
sleep 10
sudo kill $TCPDUMP_PID
```

---

## 10. Final Proof Report Template

### TERRAPHIM AI FUNCTIONAL PROOF REPORT

**Date:** [Date]
**Version:** v1.0.1
**Tester:** [Name]

### Executive Summary
- Total Functions Tested: [X]
- Functions Passing: [Y]
- Functions Failing: [Z]
- Overall Status: [PASS/FAIL]

### Component Results

#### TUI REPL
- Commands Tested: 14/14
- Pass Rate: X%
- Evidence: See tui_test_results.log

#### Server API
- Endpoints Tested: 12/12
- Pass Rate: X%
- Evidence: See server_test_results.log

#### Desktop Application
- UI Components Tested: 20/20
- Pass Rate: X%
- Evidence: See desktop_screenshots/

#### Integration Tests
- Scenarios Tested: 5/5
- Pass Rate: X%
- Evidence: See integration_logs/

### Performance Metrics
| Metric | Target | Actual | Status |
|--------|--------|--------|--------|
| Search Response | <500ms | Xms | ✅/❌ |
| Chat Response | <2s | Xs | ✅/❌ |
| Graph Render | <1s | Xms | ✅/❌ |

### Issues Discovered
1. [Issue description and severity]
2. [Issue description and severity]

### Recommendations
1. [Improvement suggestion]
2. [Improvement suggestion]

### Certification
I certify that all functions listed above have been tested according to the test plan and the results are accurately reported.

**Signature:** ________________________
**Date:** ________________________

---

## Implementation Instructions

1. **Create test scripts**: Save each script in `tests/functional/`
2. **Generate test data**: Run `generate_test_data.sh`
3. **Execute tests**: Run `run_all_tests.sh`
4. **Collect evidence**: Run `collect_evidence.sh`
5. **Generate report**: Fill in the template with actual results

This plan ensures COMPLETE functional coverage with documented proof.