# Claude Agent SDK Demos - Terraphim AI Implementation Plan

This document outlines the plan to implement each Claude Agent SDK demo using Terraphim AI and the terraphim-agent system.

## Overview

The [Claude Agent SDK Demos](https://github.com/anthropics/claude-agent-sdk-demos) repository contains 5 demonstration applications:

1. **Hello World** - Basic agent setup
2. **Research Agent** - Multi-agent coordination with web search
3. **Email Agent** - IMAP email integration
4. **Excel Demo** - Spreadsheet manipulation
5. **Simple Chat App** - Basic conversational interface

Each will be implemented using Terraphim AI's existing infrastructure:
- `terraphim_multi_agent` - Core agent system
- `terraphim_agent_supervisor` - Erlang-style supervision
- `terraphim_agent_messaging` - Inter-agent communication
- `terraphim_task_decomposition` - Task planning
- `terraphim_tui` - CLI/REPL interface

---

## Demo 1: Hello World

### Original Implementation
- TypeScript using `@anthropic-ai/claude-agent-sdk`
- Simple `query()` function returning async iterable
- Demonstrates basic agent spawning and task execution

### Terraphim Implementation Plan

**Location**: `examples/claude-agent-sdk-demos/hello-world/`

**Components**:
```
hello-world/
├── src/
│   └── main.rs          # Main binary
├── Cargo.toml           # Dependencies
└── README.md            # Documentation
```

**Implementation Steps**:

1. **Create basic agent binary** using `TerraphimAgent`:
   ```rust
   use terraphim_multi_agent::{TerraphimAgent, CommandInput, CommandType};
   use terraphim_persistence::DeviceStorage;

   #[tokio::main]
   async fn main() -> Result<(), Box<dyn std::error::Error>> {
       // Initialize storage
       DeviceStorage::init_memory_only().await?;
       let persistence = Arc::new(DeviceStorage::instance().await?);

       // Create agent with role
       let role = create_hello_world_role();
       let agent = TerraphimAgent::new(role, persistence, None).await?;
       agent.initialize().await?;

       // Execute simple query
       let input = CommandInput::new("Say hello world!".to_string(), CommandType::Generate);
       let output = agent.process_command(input).await?;

       println!("{}", output.text);
       Ok(())
   }
   ```

2. **Add streaming output** using async iterators:
   - Implement `StreamingResponse` wrapper
   - Use `tokio::sync::mpsc` for real-time token streaming

3. **Configuration options**:
   - `max_turns` - via `AgentConfig.max_command_history`
   - `model` selection - via Role's LLM provider settings
   - `cwd` - working directory context

**Terraphim Advantages**:
- Built-in persistence with `DeviceStorage`
- Role-based configuration (reusable)
- Token tracking out of the box
- Knowledge graph integration ready

**Estimated Effort**: Low complexity, 1-2 files

---

## Demo 2: Research Agent

### Original Implementation
- Multi-agent system with Lead Agent, Researcher Agents, Report-Writer Agent
- WebSearch and Write tools
- Parallel researcher spawning
- Hook-based activity tracking
- Saves research notes to `files/research_notes/`

### Terraphim Implementation Plan

**Location**: `examples/claude-agent-sdk-demos/research-agent/`

**Architecture Mapping**:

| Claude SDK Component | Terraphim Component |
|---------------------|---------------------|
| Lead Agent | `TerraphimAgent` with orchestrator role |
| Researcher Agents | Worker agents via `AgentPool` |
| Task Tool | `terraphim_task_decomposition` |
| WebSearch Tool | `haystack_*` integrations + web fetch |
| Write Tool | `CommandType::Create` with file operations |
| Hooks | `terraphim_agent_supervisor` event hooks |

**Components**:
```
research-agent/
├── src/
│   ├── main.rs              # Entry point
│   ├── lead_agent.rs        # Lead orchestrator
│   ├── researcher.rs        # Researcher worker
│   ├── report_writer.rs     # Report synthesis
│   ├── tools/
│   │   ├── web_search.rs    # Web search integration
│   │   └── file_writer.rs   # File operations
│   └── hooks.rs             # Activity tracking
├── Cargo.toml
└── README.md
```

**Implementation Steps**:

1. **Lead Agent (Orchestrator)**:
   ```rust
   use terraphim_task_decomposition::{TaskDecomposer, Task};
   use terraphim_multi_agent::workflows::MultiAgentWorkflow;

   pub struct LeadAgent {
       agent: TerraphimAgent,
       decomposer: TaskDecomposer,
   }

   impl LeadAgent {
       pub async fn plan_research(&self, topic: &str) -> Vec<Task> {
           // Decompose into 2-4 subtopics
           let tasks = self.decomposer.decompose(topic).await?;
           tasks.into_iter().take(4).collect()
       }

       pub async fn spawn_researchers(&self, tasks: Vec<Task>) -> Vec<ResearchResult> {
           // Use tokio::spawn for parallel execution
           let handles: Vec<_> = tasks.iter().map(|task| {
               tokio::spawn(async move {
                   let researcher = Researcher::new().await?;
                   researcher.research(task).await
               })
           }).collect();

           futures::future::join_all(handles).await
       }
   }
   ```

2. **Researcher Agent**:
   ```rust
   pub struct Researcher {
       agent: TerraphimAgent,
       web_client: WebSearchClient,
   }

   impl Researcher {
       pub async fn research(&self, task: &Task) -> ResearchResult {
           // Use existing haystack integrations
           let search_results = self.web_client.search(&task.query).await?;

           // Process and save notes
           let notes = self.agent.process_command(
               CommandInput::new(format!("Analyze: {:?}", search_results), CommandType::Analyze)
           ).await?;

           // Write to files/research_notes/
           self.save_notes(&task.id, &notes.text).await?;

           ResearchResult { task_id: task.id, findings: notes.text }
       }
   }
   ```

3. **Report Writer**:
   ```rust
   pub struct ReportWriter {
       agent: TerraphimAgent,
   }

   impl ReportWriter {
       pub async fn synthesize(&self, results: Vec<ResearchResult>) -> Report {
           // Read all research notes using Glob pattern
           let notes = self.read_research_notes().await?;

           // Generate comprehensive report
           let report = self.agent.process_command(
               CommandInput::new(format!("Synthesize report from: {:?}", notes), CommandType::Create)
           ).await?;

           // Save to files/reports/
           self.save_report(&report.text).await?;

           Report { content: report.text }
       }
   }
   ```

4. **Activity Tracking with Hooks**:
   ```rust
   use terraphim_agent_supervisor::{SupervisorConfig, RestartPolicy};

   pub struct ActivityTracker {
       logs: Arc<RwLock<Vec<ActivityLog>>>,
   }

   impl ActivityTracker {
       pub fn on_tool_call(&self, agent_id: &AgentId, tool: &str, input: &str) {
           self.logs.write().await.push(ActivityLog {
               timestamp: Utc::now(),
               agent_id: agent_id.clone(),
               tool: tool.to_string(),
               input: input.to_string(),
               status: "started".to_string(),
           });
       }
   }
   ```

5. **Main Workflow**:
   ```rust
   #[tokio::main]
   async fn main() -> Result<()> {
       let lead = LeadAgent::new().await?;
       let tracker = ActivityTracker::new();

       // User research request
       let topic = "Impact of quantum computing on cryptography";

       // Step 1: Plan research
       let tasks = lead.plan_research(topic).await?;

       // Step 2: Spawn parallel researchers
       let results = lead.spawn_researchers(tasks).await?;

       // Step 3: Generate report
       let writer = ReportWriter::new().await?;
       let report = writer.synthesize(results).await?;

       // Save activity log
       tracker.save_logs().await?;

       println!("Report: {}", report.content);
       Ok(())
   }
   ```

**Web Search Integration Options**:
- Use existing `haystack_*` crates for structured sources
- Add new `WebSearchHaystack` using `reqwest` + search API
- Integrate with MCP server's web fetch capabilities

**Terraphim Advantages**:
- Existing `terraphim_task_decomposition` for breaking down research
- `AgentPool` for managing researcher lifecycle
- `terraphim_agent_supervisor` for fault-tolerant parallel execution
- Knowledge graph can store research findings for semantic search
- Built-in token/cost tracking per researcher

**Estimated Effort**: High complexity, 5-8 files

---

## Demo 3: Email Agent

### Original Implementation
- IMAP email access
- AI-powered inbox display and search
- Agentic email assistance
- Requires email credentials in environment

### Terraphim Implementation Plan

**Location**: `examples/claude-agent-sdk-demos/email-agent/`

**Architecture Mapping**:

| Claude SDK Component | Terraphim Component |
|---------------------|---------------------|
| IMAP Client | New `haystack_imap` crate or standalone client |
| Email Search | `TerraphimAgent` with `CommandType::Search` |
| Email Display | TUI integration via `terraphim_tui` |
| AI Assistance | Agent commands for summarization/reply drafting |

**Components**:
```
email-agent/
├── src/
│   ├── main.rs              # Entry point with CLI
│   ├── imap_client.rs       # IMAP connection handling
│   ├── email_agent.rs       # AI-powered email operations
│   ├── commands/
│   │   ├── inbox.rs         # Display inbox
│   │   ├── search.rs        # Agentic search
│   │   ├── summarize.rs     # Email summarization
│   │   └── reply.rs         # Draft reply generation
│   └── tui.rs               # Optional TUI interface
├── Cargo.toml
└── README.md
```

**Implementation Steps**:

1. **IMAP Client**:
   ```rust
   use async_imap::Session;
   use async_native_tls::TlsStream;
   use tokio::net::TcpStream;

   pub struct ImapEmailClient {
       session: Session<TlsStream<TcpStream>>,
   }

   impl ImapEmailClient {
       pub async fn connect(config: &EmailConfig) -> Result<Self> {
           let tls = async_native_tls::TlsConnector::new();
           let stream = TcpStream::connect(&config.imap_server).await?;
           let tls_stream = tls.connect(&config.imap_server, stream).await?;

           let client = async_imap::connect(tls_stream).await?;
           let session = client.login(&config.username, &config.password).await?;

           Ok(Self { session })
       }

       pub async fn fetch_inbox(&mut self, limit: usize) -> Vec<Email> {
           self.session.select("INBOX").await?;
           let messages = self.session.fetch("1:*", "RFC822").await?;
           // Parse and return emails
       }

       pub async fn search(&mut self, query: &str) -> Vec<Email> {
           let criteria = format!("OR SUBJECT \"{}\" BODY \"{}\"", query, query);
           self.session.search(&criteria).await?
       }
   }
   ```

2. **Email Agent**:
   ```rust
   pub struct EmailAgent {
       agent: TerraphimAgent,
       imap: ImapEmailClient,
   }

   impl EmailAgent {
       pub async fn display_inbox(&self) -> Result<String> {
           let emails = self.imap.fetch_inbox(20).await?;

           let summary = self.agent.process_command(
               CommandInput::new(
                   format!("Summarize these emails for display:\n{:?}", emails),
                   CommandType::Analyze
               )
           ).await?;

           Ok(summary.text)
       }

       pub async fn agentic_search(&self, query: &str) -> Result<Vec<SearchResult>> {
           // First, let agent understand the query
           let refined_query = self.agent.process_command(
               CommandInput::new(
                   format!("Convert this to email search terms: {}", query),
                   CommandType::Generate
               )
           ).await?;

           // Perform IMAP search
           let emails = self.imap.search(&refined_query.text).await?;

           // Agent ranks and explains results
           let analysis = self.agent.process_command(
               CommandInput::new(
                   format!("Analyze relevance of these emails to '{}': {:?}", query, emails),
                   CommandType::Analyze
               )
           ).await?;

           Ok(parse_search_results(&analysis.text))
       }

       pub async fn draft_reply(&self, email_id: &str) -> Result<String> {
           let email = self.imap.fetch_email(email_id).await?;

           let draft = self.agent.process_command(
               CommandInput::new(
                   format!("Draft a professional reply to:\n{}", email.body),
                   CommandType::Create
               )
           ).await?;

           Ok(draft.text)
       }
   }
   ```

3. **CLI Interface** (using `clap`):
   ```rust
   #[derive(Parser)]
   struct Cli {
       #[command(subcommand)]
       command: Commands,
   }

   #[derive(Subcommand)]
   enum Commands {
       Inbox { #[arg(short, default_value = "10")] limit: usize },
       Search { query: String },
       Summarize { email_id: String },
       Reply { email_id: String },
   }
   ```

4. **Alternative: JMAP Integration**:
   - Use existing `haystack_jmap` crate for modern email protocol
   - Better async support than IMAP
   - Structured JSON responses

**Security Considerations**:
- Use `terraphim_onepassword_cli` for credential management
- Environment variable fallback
- Never log credentials

**Terraphim Advantages**:
- Existing `haystack_jmap` crate for email integration
- `terraphim_onepassword_cli` for secure credential storage
- TUI integration ready via `terraphim_tui`
- Knowledge graph can index important emails for semantic search

**Estimated Effort**: Medium complexity, 4-6 files

---

## Demo 4: Excel Demo

### Original Implementation
- Electron + React desktop app
- Python openpyxl for Excel manipulation
- AI-powered spreadsheet creation/analysis
- Examples: Workout Tracker, Budget Tracker

### Terraphim Implementation Plan

**Location**: `examples/claude-agent-sdk-demos/excel-demo/`

**Architecture Mapping**:

| Claude SDK Component | Terraphim Component |
|---------------------|---------------------|
| Electron/React UI | Tauri desktop app or CLI |
| Python openpyxl | Rust `calamine` (read) + `rust_xlsxwriter` (write) |
| AI Analysis | `TerraphimAgent` with `CommandType::Analyze` |
| File Operations | Standard Rust file I/O |

**Components**:
```
excel-demo/
├── src/
│   ├── main.rs              # Entry point
│   ├── excel/
│   │   ├── reader.rs        # Read Excel files
│   │   ├── writer.rs        # Write Excel files
│   │   └── formatter.rs     # Styling and formatting
│   ├── agent.rs             # AI-powered operations
│   └── templates/
│       ├── budget.rs        # Budget tracker template
│       └── workout.rs       # Workout tracker template
├── Cargo.toml
└── README.md
```

**Implementation Steps**:

1. **Excel Reader** (using `calamine`):
   ```rust
   use calamine::{Reader, open_workbook, Xlsx, DataType};

   pub struct ExcelReader;

   impl ExcelReader {
       pub fn read_workbook(path: &Path) -> Result<Workbook> {
           let mut workbook: Xlsx<_> = open_workbook(path)?;

           let sheets: Vec<Sheet> = workbook.sheet_names()
               .iter()
               .map(|name| {
                   let range = workbook.worksheet_range(name)?;
                   Sheet {
                       name: name.clone(),
                       data: range.rows().map(|row| row.to_vec()).collect(),
                   }
               })
               .collect();

           Ok(Workbook { sheets })
       }

       pub fn to_markdown(&self, workbook: &Workbook) -> String {
           // Convert to markdown table for AI analysis
       }
   }
   ```

2. **Excel Writer** (using `rust_xlsxwriter`):
   ```rust
   use rust_xlsxwriter::{Workbook, Worksheet, Format};

   pub struct ExcelWriter {
       workbook: Workbook,
   }

   impl ExcelWriter {
       pub fn new() -> Self {
           Self { workbook: Workbook::new() }
       }

       pub fn create_sheet(&mut self, name: &str, data: Vec<Vec<String>>) -> Result<()> {
           let sheet = self.workbook.add_worksheet();
           sheet.set_name(name)?;

           for (row_idx, row) in data.iter().enumerate() {
               for (col_idx, cell) in row.iter().enumerate() {
                   sheet.write_string(row_idx as u32, col_idx as u16, cell)?;
               }
           }
           Ok(())
       }

       pub fn add_formula(&mut self, sheet: &str, cell: &str, formula: &str) -> Result<()> {
           // Add Excel formula
       }

       pub fn apply_styling(&mut self, sheet: &str, range: &str, style: &Style) -> Result<()> {
           let format = Format::new()
               .set_bold()
               .set_bg_color(style.bg_color)
               .set_border(style.border);
           // Apply format to range
       }

       pub fn save(&self, path: &Path) -> Result<()> {
           self.workbook.save(path)?;
           Ok(())
       }
   }
   ```

3. **Excel Agent**:
   ```rust
   pub struct ExcelAgent {
       agent: TerraphimAgent,
       reader: ExcelReader,
       writer: ExcelWriter,
   }

   impl ExcelAgent {
       pub async fn analyze(&self, path: &Path) -> Result<Analysis> {
           let workbook = self.reader.read_workbook(path)?;
           let markdown = self.reader.to_markdown(&workbook);

           let analysis = self.agent.process_command(
               CommandInput::new(
                   format!("Analyze this spreadsheet data:\n{}", markdown),
                   CommandType::Analyze
               )
           ).await?;

           Ok(Analysis { summary: analysis.text })
       }

       pub async fn generate_workbook(&self, description: &str) -> Result<PathBuf> {
           // Step 1: Agent plans the structure
           let structure = self.agent.process_command(
               CommandInput::new(
                   format!("Design an Excel workbook structure for: {}\nOutput as JSON with sheets, columns, formulas", description),
                   CommandType::Create
               )
           ).await?;

           let spec: WorkbookSpec = serde_json::from_str(&structure.text)?;

           // Step 2: Create the workbook
           let mut writer = ExcelWriter::new();
           for sheet in spec.sheets {
               writer.create_sheet(&sheet.name, sheet.data)?;
               for formula in sheet.formulas {
                   writer.add_formula(&sheet.name, &formula.cell, &formula.expression)?;
               }
           }

           let path = PathBuf::from(format!("{}.xlsx", spec.name));
           writer.save(&path)?;

           Ok(path)
       }

       pub async fn create_budget_tracker(&self) -> Result<PathBuf> {
           self.generate_workbook("Budget tracker with income, expenses, categories, monthly totals, and charts").await
       }

       pub async fn create_workout_tracker(&self) -> Result<PathBuf> {
           self.generate_workbook("Workout tracker with exercises, sets, reps, weight, progress tracking").await
       }
   }
   ```

4. **CLI Interface**:
   ```rust
   #[derive(Parser)]
   struct Cli {
       #[command(subcommand)]
       command: Commands,
   }

   #[derive(Subcommand)]
   enum Commands {
       Analyze { path: PathBuf },
       Create { description: String, #[arg(short)] output: PathBuf },
       Template { #[arg(value_enum)] template: TemplateType },
   }

   #[derive(ValueEnum, Clone)]
   enum TemplateType {
       Budget,
       Workout,
       Invoice,
       ProjectPlan,
   }
   ```

**Optional: Desktop GUI Integration**:
- Use existing Tauri infrastructure in `desktop/`
- Add Excel component to Svelte frontend
- File picker for upload/download

**Terraphim Advantages**:
- Pure Rust implementation (no Python dependency)
- Can integrate with knowledge graph for data validation
- Role-based templates for different use cases
- Cross-platform compatibility

**Estimated Effort**: Medium complexity, 4-5 files

---

## Demo 5: Simple Chat App

### Original Implementation
- Basic conversational interface
- Likely uses streaming responses
- Simple message history management

### Terraphim Implementation Plan

**Location**: `examples/claude-agent-sdk-demos/simple-chat/`

**Architecture Mapping**:

| Claude SDK Component | Terraphim Component |
|---------------------|---------------------|
| Chat Interface | `terraphim_tui` REPL or new CLI |
| Message History | Agent's `CommandHistory` |
| Streaming | `tokio::sync::mpsc` channels |
| Context Management | `AgentContext` with `ContextItem`s |

**Components**:
```
simple-chat/
├── src/
│   ├── main.rs              # Entry point
│   ├── chat.rs              # Chat session management
│   ├── history.rs           # Message history
│   └── streaming.rs         # Streaming response handler
├── Cargo.toml
└── README.md
```

**Implementation Steps**:

1. **Chat Session**:
   ```rust
   pub struct ChatSession {
       agent: TerraphimAgent,
       history: Vec<Message>,
       system_prompt: String,
   }

   impl ChatSession {
       pub async fn new(system_prompt: Option<String>) -> Result<Self> {
           let persistence = Arc::new(DeviceStorage::arc_memory_only().await?);
           let role = create_chat_role(system_prompt.as_deref());
           let agent = TerraphimAgent::new(role, persistence, None).await?;
           agent.initialize().await?;

           Ok(Self {
               agent,
               history: Vec::new(),
               system_prompt: system_prompt.unwrap_or_default(),
           })
       }

       pub async fn send_message(&mut self, content: &str) -> Result<String> {
           // Add user message to history
           self.history.push(Message::user(content.to_string()));

           // Build context from history
           let context = self.build_context();

           // Get response
           let input = CommandInput::new(context, CommandType::Generate);
           let output = self.agent.process_command(input).await?;

           // Add assistant response to history
           self.history.push(Message::assistant(output.text.clone()));

           Ok(output.text)
       }

       fn build_context(&self) -> String {
           let mut context = String::new();
           if !self.system_prompt.is_empty() {
               context.push_str(&format!("System: {}\n\n", self.system_prompt));
           }
           for msg in &self.history {
               context.push_str(&format!("{}: {}\n", msg.role, msg.content));
           }
           context
       }
   }
   ```

2. **Streaming Responses**:
   ```rust
   pub struct StreamingChat {
       session: ChatSession,
       tx: mpsc::Sender<String>,
       rx: mpsc::Receiver<String>,
   }

   impl StreamingChat {
       pub async fn send_streaming(&mut self, content: &str) -> impl Stream<Item = String> {
           let (tx, rx) = mpsc::channel(100);

           // Spawn task for streaming
           let agent = self.session.agent.clone();
           tokio::spawn(async move {
               // Stream tokens as they arrive
               // This requires LLM provider streaming support
           });

           ReceiverStream::new(rx)
       }
   }
   ```

3. **Interactive REPL**:
   ```rust
   use rustyline::Editor;

   #[tokio::main]
   async fn main() -> Result<()> {
       let mut session = ChatSession::new(None).await?;
       let mut rl = Editor::<()>::new()?;

       println!("Simple Chat - Type 'quit' to exit");
       println!("================================");

       loop {
           let readline = rl.readline("You: ");
           match readline {
               Ok(line) if line.trim() == "quit" => break,
               Ok(line) => {
                   let response = session.send_message(&line).await?;
                   println!("Assistant: {}", response);
               }
               Err(_) => break,
           }
       }

       Ok(())
   }
   ```

4. **Alternative: TUI Integration**:
   - Leverage existing `terraphim_tui` infrastructure
   - Add `/chat` command to existing REPL
   - Use ratatui for prettier output

**Terraphim Advantages**:
- Existing REPL infrastructure in `terraphim_tui`
- Built-in command history with persistence
- Token tracking and cost monitoring
- Can integrate semantic search for context retrieval

**Estimated Effort**: Low complexity, 2-3 files

---

## Implementation Priority

### Phase 1: Foundation (Hello World + Simple Chat)
1. **Hello World** - Establish patterns, test agent creation
2. **Simple Chat** - Build on hello world, add conversation management

### Phase 2: Multi-Agent (Research Agent)
3. **Research Agent** - Full multi-agent coordination
   - Demonstrates all workflow patterns
   - Uses supervision and messaging

### Phase 3: Integrations (Email + Excel)
4. **Email Agent** - External service integration
5. **Excel Demo** - File format handling

---

## Shared Infrastructure

### Common Dependencies
```toml
[dependencies]
terraphim_multi_agent = { path = "../../crates/terraphim_multi_agent" }
terraphim_persistence = { path = "../../crates/terraphim_persistence" }
terraphim_config = { path = "../../crates/terraphim_config" }
terraphim_types = { path = "../../crates/terraphim_types" }
tokio = { version = "1", features = ["full"] }
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
```

### Shared Utilities
```
examples/claude-agent-sdk-demos/
├── shared/
│   ├── role_factory.rs      # Role creation helpers
│   ├── streaming.rs         # Streaming response utilities
│   └── config.rs            # Common configuration
```

---

## Testing Strategy

Each demo should include:

1. **Unit Tests**: Test individual components
2. **Integration Tests**: Test agent interactions
3. **E2E Tests**: Full workflow validation

```rust
#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn test_hello_world() {
        // Test basic agent creation and response
    }

    #[tokio::test]
    #[ignore] // Requires Ollama running
    async fn test_live_llm_response() {
        // Test with actual LLM
    }
}
```

---

## Success Criteria

Each demo is complete when:

1. Functionally equivalent to Claude SDK version
2. Uses idiomatic Terraphim patterns
3. Has working tests
4. Has clear documentation
5. Can run standalone or integrate with main system

---

## Next Steps

1. Create directory structure
2. Implement Hello World demo
3. Add Simple Chat demo
4. Implement Research Agent (most complex)
5. Add Email and Excel demos
6. Create integration tests
7. Update main README with demo links
