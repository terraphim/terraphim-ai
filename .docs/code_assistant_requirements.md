# Code Assistant Requirements: Superior AI Programming Tool

**Version:** 1.0
**Date:** 2025-10-29
**Objective:** Build a coding assistant that surpasses claude-code, aider, and opencode by combining their best features

---

## Executive Summary

This document specifies requirements for an advanced AI coding assistant that combines the strengths of three leading tools:

- **Claude Code**: Plugin system, multi-agent orchestration, confidence scoring, event hooks
- **Aider**: Text-based edit fallback, RepoMap context management, robust fuzzy matching
- **OpenCode**: Built-in LSP integration, 9-strategy edit matching, client/server architecture

**Key Innovation**: Layer multiple approaches instead of choosing one. Start with tools (fastest), fall back to fuzzy matching (most reliable), validate with LSP (most immediate), recover with git (most forgiving).

---

## 1. Mandatory Features

These features are non-negotiable requirements:

### 1.1 Multi-Strategy Edit Application (from Aider)
**Requirement**: Must apply edits to files even when the model doesn't support tool calls.

**Implementation**: Text-based SEARCH/REPLACE parser with multiple fallback strategies:

```python
# Aider's approach - parse from LLM text output
"""
<<<<<<< SEARCH
old_code_here
=======
new_code_here
>>>>>>> REPLACE
"""
```

**Success Criteria**:
- Works with any LLM (GPT-3.5, GPT-4, Claude, local models)
- No tool/function calling required
- Robust parsing from natural language responses

### 1.2 Pre-Tool and Post-Tool Checks (from Claude Code)
**Requirement**: Validation hooks before and after every tool execution.

**Implementation**: Event-driven hook system:

```typescript
// Pre-tool validation
hooks.on('PreToolUse', async (tool, params) => {
  // Permission check
  if (!permissions.allows(tool.name, params)) {
    throw new PermissionDenied(tool.name);
  }

  // File existence check
  if (tool.name === 'edit' && !fs.existsSync(params.file_path)) {
    throw new FileNotFound(params.file_path);
  }

  // Custom validators from config
  await runCustomValidators('pre-tool', tool, params);
});

// Post-tool validation
hooks.on('PostToolUse', async (tool, params, result) => {
  // LSP diagnostics
  if (tool.name === 'edit') {
    const diagnostics = await lsp.check(params.file_path);
    if (diagnostics.errors.length > 0) {
      await autoFix(params.file_path, diagnostics);
    }
  }

  // Auto-lint
  if (config.autoLint) {
    await runLinter(params.file_path);
  }

  // Custom validators
  await runCustomValidators('post-tool', tool, params, result);
});
```

**Success Criteria**:
- Every tool call intercepted
- Failures prevent tool execution (pre-tool) or trigger recovery (post-tool)
- Extensible via configuration

### 1.3 Pre-LLM and Post-LLM Validation
**Requirement**: Additional validation layers around LLM interactions.

**Implementation**:

```python
class LLMPipeline:
    def __init__(self):
        self.pre_validators = []
        self.post_validators = []

    async def call_llm(self, messages, context):
        # PRE-LLM VALIDATION
        validated_context = await self.pre_llm_validation(messages, context)

        # Include validated context
        enriched_messages = self.enrich_with_context(messages, validated_context)

        # Call LLM
        response = await self.llm_provider.complete(enriched_messages)

        # POST-LLM VALIDATION
        validated_response = await self.post_llm_validation(response, context)

        return validated_response

    async def pre_llm_validation(self, messages, context):
        """Validate and enrich context before LLM call"""
        validators = [
            self.validate_file_references,      # Files mentioned exist
            self.validate_context_size,         # Within token limits
            self.validate_permissions,          # Has access to mentioned files
            self.enrich_with_repo_map,         # Add code structure
            self.check_cache_freshness,        # Context not stale
        ]

        result = context
        for validator in validators:
            result = await validator(messages, result)

        return result

    async def post_llm_validation(self, response, context):
        """Validate LLM output before execution"""
        validators = [
            self.parse_tool_calls,              # Extract structured actions
            self.validate_file_paths,           # Paths are valid
            self.check_confidence_threshold,    # ≥80 for code review
            self.validate_code_syntax,          # Basic syntax check
            self.check_security_patterns,       # No obvious vulnerabilities
        ]

        result = response
        for validator in validators:
            result = await validator(result, context)

        return result
```

**Success Criteria**:
- Context validated before every LLM call
- Output validated before execution
- Token limits respected
- Security patterns checked

---

## 2. Architecture & Design Patterns

### 2.1 Overall Architecture

**Pattern**: Client/Server + Plugin System + Multi-Agent Orchestration

```
┌─────────────────────────────────────────────────────────────┐
│                      CLIENT LAYER                            │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │   CLI    │  │   TUI    │  │   Web    │  │  Mobile  │   │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘   │
└────────────────────────┬────────────────────────────────────┘
                         │ HTTP/SSE/WebSocket
┌────────────────────────▼────────────────────────────────────┐
│                      SERVER LAYER                            │
│  ┌───────────────────────────────────────────────────────┐  │
│  │              Session Manager                          │  │
│  │  - Conversation state                                 │  │
│  │  - Context management                                 │  │
│  │  - Snapshot system                                    │  │
│  └───────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌───────────────────────────────────────────────────────┐  │
│  │              Agent Orchestrator                       │  │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐           │  │
│  │  │ Main     │  │ Debugger │  │ Reviewer │  + More   │  │
│  │  │ Agent    │  │ Agent    │  │ Agent    │           │  │
│  │  └──────────┘  └──────────┘  └──────────┘           │  │
│  │       │              │              │                 │  │
│  │       └──────────────┴──────────────┘                │  │
│  │                      │                                │  │
│  │         ┌────────────▼──────────────┐                │  │
│  │         │   Parallel Execution      │                │  │
│  │         └───────────────────────────┘                │  │
│  └───────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌───────────────────────────────────────────────────────┐  │
│  │              LLM Pipeline                             │  │
│  │  ┌─────────────┐  ┌─────────┐  ┌──────────────┐     │  │
│  │  │ Pre-LLM     │─→│   LLM   │─→│  Post-LLM    │     │  │
│  │  │ Validation  │  │  Call   │  │  Validation  │     │  │
│  │  └─────────────┘  └─────────┘  └──────────────┘     │  │
│  └───────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌───────────────────────────────────────────────────────┐  │
│  │              Tool Execution Layer                     │  │
│  │  ┌─────────────┐  ┌─────────┐  ┌──────────────┐     │  │
│  │  │ Pre-Tool    │─→│  Tool   │─→│  Post-Tool   │     │  │
│  │  │ Validation  │  │  Exec   │  │  Validation  │     │  │
│  │  └─────────────┘  └─────────┘  └──────────────┘     │  │
│  └───────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌───────────────────────────────────────────────────────┐  │
│  │              Core Services                            │  │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌─────────┐ │  │
│  │  │ RepoMap  │ │   LSP    │ │  Linter  │ │   Git   │ │  │
│  │  └──────────┘ └──────────┘ └──────────┘ └─────────┘ │  │
│  └───────────────────────────────────────────────────────┘  │
│                                                              │
│  ┌───────────────────────────────────────────────────────┐  │
│  │              Plugin System                            │  │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐             │  │
│  │  │  Hooks   │ │Commands  │ │  Tools   │             │  │
│  │  └──────────┘ └──────────┘ └──────────┘             │  │
│  └───────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────┘
```

**Key Design Decisions**:

1. **Client/Server Split** (OpenCode approach)
   - Enables multiple frontends (CLI, TUI, Web, Mobile)
   - Remote execution support
   - State persistence on server
   - API-first design

2. **Plugin Architecture** (Claude Code approach)
   - Commands: User-facing slash commands
   - Agents: Specialized AI assistants
   - Hooks: Event-driven automation
   - Tools: Low-level operations

3. **Multi-Agent System** (Claude Code approach)
   - Specialized agents with focused prompts
   - Parallel execution for independent tasks
   - Agent isolation prevents context pollution
   - Confidence scoring for quality control

### 2.2 Four-Layer Validation Pipeline

**Critical Design**: Every operation passes through multiple validation layers.

```
┌────────────────────────────────────────────────────────────┐
│                    USER REQUEST                            │
└───────────────────────┬────────────────────────────────────┘
                        │
          ┌─────────────▼─────────────┐
          │   LAYER 1: PRE-LLM        │
          │   Validation              │
          │   ─────────────────       │
          │   • Context validation    │
          │   • Token budget check    │
          │   • Permission check      │
          │   • File existence        │
          │   • RepoMap enrichment    │
          └─────────────┬─────────────┘
                        │
          ┌─────────────▼─────────────┐
          │   LLM CALL                │
          └─────────────┬─────────────┘
                        │
          ┌─────────────▼─────────────┐
          │   LAYER 2: POST-LLM       │
          │   Validation              │
          │   ─────────────────       │
          │   • Parse tool calls      │
          │   • Validate paths        │
          │   • Confidence check      │
          │   • Syntax validation     │
          │   • Security scan         │
          └─────────────┬─────────────┘
                        │
          ┌─────────────▼─────────────┐
          │   LAYER 3: PRE-TOOL       │
          │   Validation              │
          │   ─────────────────       │
          │   • Permission check      │
          │   • File time assertion   │
          │   • Hook: PreToolUse      │
          │   • Dry-run validation    │
          └─────────────┬─────────────┘
                        │
          ┌─────────────▼─────────────┐
          │   TOOL EXECUTION          │
          └─────────────┬─────────────┘
                        │
          ┌─────────────▼─────────────┐
          │   LAYER 4: POST-TOOL      │
          │   Validation              │
          │   ─────────────────       │
          │   • LSP diagnostics       │
          │   • Linter execution      │
          │   • Test execution        │
          │   • Hook: PostToolUse     │
          │   • Git commit            │
          │   • Diff generation       │
          └─────────────┬─────────────┘
                        │
          ┌─────────────▼─────────────┐
          │   ERROR RECOVERY          │
          │   (if validation fails)   │
          │   ─────────────────       │
          │   • Rollback via git      │
          │   • Restore snapshot      │
          │   • Retry with fixes      │
          │   • User notification     │
          └───────────────────────────┘
```

**Implementation Details**:

```typescript
class ValidationPipeline {
  // LAYER 1: PRE-LLM
  async validatePreLLM(context: Context): Promise<ValidatedContext> {
    // 1. Check token budget
    const tokenCount = this.estimateTokens(context);
    if (tokenCount > context.model.maxTokens) {
      context = await this.compactContext(context);
    }

    // 2. Validate file references
    for (const file of context.files) {
      if (!fs.existsSync(file)) {
        throw new ValidationError(`File not found: ${file}`);
      }
    }

    // 3. Check permissions
    await this.permissionManager.check(context.requestedActions);

    // 4. Enrich with RepoMap
    context.repoMap = await this.repoMap.generate(context.files);

    // 5. Check cache freshness
    if (this.cache.isStale(context)) {
      await this.cache.refresh(context);
    }

    return context;
  }

  // LAYER 2: POST-LLM
  async validatePostLLM(response: LLMResponse): Promise<ValidatedResponse> {
    // 1. Parse tool calls (including text-based fallback)
    const actions = await this.parseActions(response);

    // 2. Validate file paths
    for (const action of actions) {
      if (action.type === 'edit') {
        this.validatePath(action.file_path);
      }
    }

    // 3. Confidence check
    if (response.type === 'code_review') {
      const confidence = this.calculateConfidence(response);
      if (confidence < 0.8) {
        // Filter low-confidence feedback
        response = this.filterLowConfidence(response);
      }
    }

    // 4. Basic syntax validation
    for (const action of actions) {
      if (action.type === 'edit' && action.new_code) {
        await this.validateSyntax(action.file_path, action.new_code);
      }
    }

    // 5. Security scan
    await this.securityScanner.scan(actions);

    return { response, actions };
  }

  // LAYER 3: PRE-TOOL
  async validatePreTool(tool: Tool, params: any): Promise<void> {
    // 1. Permission check
    const allowed = await this.permissionManager.allows(tool.name, params);
    if (!allowed) {
      throw new PermissionDenied(`Tool ${tool.name} not allowed`);
    }

    // 2. File time assertion (detect external changes)
    if (params.file_path) {
      const currentTime = fs.statSync(params.file_path).mtime;
      const knownTime = this.fileTime.get(params.file_path);
      if (knownTime && currentTime > knownTime) {
        throw new FileChangedError(`${params.file_path} modified externally`);
      }
    }

    // 3. Run pre-tool hooks
    await this.hooks.emit('PreToolUse', tool, params);

    // 4. Dry-run validation (if supported)
    if (tool.supportsDryRun) {
      await tool.dryRun(params);
    }
  }

  // LAYER 4: POST-TOOL
  async validatePostTool(tool: Tool, params: any, result: any): Promise<void> {
    // 1. LSP diagnostics
    if (tool.name === 'edit' && params.file_path) {
      const diagnostics = await this.lsp.check(params.file_path);

      if (diagnostics.errors.length > 0) {
        // Attempt auto-fix
        const fixed = await this.autoFix(params.file_path, diagnostics);
        if (!fixed) {
          throw new ValidationError(`LSP errors: ${diagnostics.errors}`);
        }
      }
    }

    // 2. Run linter
    if (this.config.autoLint && params.file_path) {
      const lintResult = await this.linter.lint(params.file_path);
      if (lintResult.fatal.length > 0) {
        throw new ValidationError(`Lint errors: ${lintResult.fatal}`);
      }
    }

    // 3. Run tests (if configured)
    if (this.config.autoTest) {
      const testResult = await this.testRunner.runRelated(params.file_path);
      if (!testResult.success) {
        throw new ValidationError(`Tests failed: ${testResult.failures}`);
      }
    }

    // 4. Run post-tool hooks
    await this.hooks.emit('PostToolUse', tool, params, result);

    // 5. Git commit (for rollback)
    if (this.config.autoCommit) {
      const diff = this.generateDiff(params.file_path);
      await this.git.commit(params.file_path, diff);
    }

    // 6. Update file time tracking
    if (params.file_path) {
      this.fileTime.update(params.file_path);
    }
  }
}
```

---

## 3. File Editing System

### 3.1 Hybrid Multi-Strategy Approach

**Design Philosophy**: Layer multiple strategies for maximum reliability.

```
┌─────────────────────────────────────────────────────────┐
│  STRATEGY 1: Tool-based Edit (Primary - Fastest)       │
│  ─────────────────────────────────────────────────      │
│  • Uses native Edit/Patch tools                        │
│  • Direct API calls                                     │
│  • Most efficient                                       │
│  ✓ Try first if tools available                        │
└────────────┬────────────────────────────────────────────┘
             │ (on failure or no tool support)
             ▼
┌─────────────────────────────────────────────────────────┐
│  STRATEGY 2: Text-based SEARCH/REPLACE (Fallback)      │
│  ─────────────────────────────────────────────────      │
│  • Parse from LLM text output                          │
│  • Works without tool support                          │
│  • Multiple sub-strategies:                            │
│    1. Exact match                                      │
│    2. Whitespace-flexible                              │
│    3. Block anchor match                               │
│    4. Levenshtein fuzzy match                          │
│    5. Context-aware match                              │
│    6. Dotdotdot handling                               │
│  ✓ Try each until one succeeds                        │
└────────────┬────────────────────────────────────────────┘
             │ (on all failures)
             ▼
┌─────────────────────────────────────────────────────────┐
│  STRATEGY 3: Unified Diff/Patch (Advanced)             │
│  ─────────────────────────────────────────────────      │
│  • Parse unified diff format                           │
│  • Apply with fuzz factor                              │
│  • Context-based matching                              │
│  ✓ Try if diff format detected                        │
└────────────┬────────────────────────────────────────────┘
             │ (on all failures)
             ▼
┌─────────────────────────────────────────────────────────┐
│  STRATEGY 4: Whole File Rewrite (Last Resort)          │
│  ─────────────────────────────────────────────────      │
│  • Replace entire file contents                        │
│  • Generate diff for review                            │
│  • Most token-intensive                                │
│  ✓ Always succeeds                                     │
└─────────────────────────────────────────────────────────┘
```

### 3.2 Detailed Strategy Implementations

#### Strategy 1: Tool-Based Edit

```typescript
class ToolBasedEditor {
  async edit(file_path: string, old_string: string, new_string: string): Promise<EditResult> {
    try {
      // Use native Edit tool
      const result = await this.tools.edit({
        file_path,
        old_string,
        new_string
      });

      return {
        success: true,
        strategy: 'tool-based',
        result
      };
    } catch (error) {
      // Fall back to next strategy
      throw new StrategyFailed('tool-based', error);
    }
  }
}
```

#### Strategy 2: Text-Based SEARCH/REPLACE (Aider Approach)

```python
class SearchReplaceEditor:
    """Parse SEARCH/REPLACE blocks from LLM text output"""

    def parse_blocks(self, text: str) -> List[EditBlock]:
        """Extract all SEARCH/REPLACE blocks"""
        pattern = r'<<<<<<< SEARCH\n(.*?)\n=======\n(.*?)\n>>>>>>> REPLACE'
        matches = re.findall(pattern, text, re.DOTALL)

        blocks = []
        for search, replace in matches:
            # Look back 3 lines for filename
            filename = self.find_filename(text, search)
            blocks.append(EditBlock(filename, search, replace))

        return blocks

    def apply_edit(self, file_path: str, search: str, replace: str) -> EditResult:
        """Apply edit with multiple fallback strategies"""
        content = read_file(file_path)

        # Strategy 2.1: Exact match
        result = self.exact_match(content, search, replace)
        if result:
            return self.write_result(file_path, result, 'exact-match')

        # Strategy 2.2: Whitespace-flexible match
        result = self.whitespace_flexible(content, search, replace)
        if result:
            return self.write_result(file_path, result, 'whitespace-flexible')

        # Strategy 2.3: Block anchor match (first/last lines)
        result = self.block_anchor_match(content, search, replace)
        if result:
            return self.write_result(file_path, result, 'block-anchor')

        # Strategy 2.4: Levenshtein fuzzy match
        result = self.fuzzy_match(content, search, replace, threshold=0.8)
        if result:
            return self.write_result(file_path, result, 'fuzzy-match')

        # Strategy 2.5: Context-aware match
        result = self.context_aware_match(content, search, replace)
        if result:
            return self.write_result(file_path, result, 'context-aware')

        # Strategy 2.6: Dotdotdot handling (elided code)
        result = self.dotdotdot_match(content, search, replace)
        if result:
            return self.write_result(file_path, result, 'dotdotdot')

        # All strategies failed
        raise EditFailed(self.suggest_similar(content, search))

    def exact_match(self, content: str, search: str, replace: str) -> Optional[str]:
        """Strategy 2.1: Perfect string match"""
        if search in content:
            return content.replace(search, replace, 1)  # Replace first occurrence
        return None

    def whitespace_flexible(self, content: str, search: str, replace: str) -> Optional[str]:
        """Strategy 2.2: Match ignoring leading/trailing whitespace per line"""
        content_lines = content.splitlines()
        search_lines = search.splitlines()
        replace_lines = replace.splitlines()

        # Try to find search block with flexible whitespace
        for i in range(len(content_lines) - len(search_lines) + 1):
            if self.lines_match_flexible(content_lines[i:i+len(search_lines)], search_lines):
                # Found match - preserve original indentation
                indentation = self.get_indentation(content_lines[i])
                replaced = self.apply_indentation(replace_lines, indentation)

                new_content = (
                    content_lines[:i] +
                    replaced +
                    content_lines[i+len(search_lines):]
                )
                return '\n'.join(new_content)

        return None

    def block_anchor_match(self, content: str, search: str, replace: str) -> Optional[str]:
        """Strategy 2.3: Match using first and last lines as anchors"""
        search_lines = search.splitlines()
        if len(search_lines) < 2:
            return None  # Need at least 2 lines for anchors

        first_line = search_lines[0].strip()
        last_line = search_lines[-1].strip()

        content_lines = content.splitlines()
        candidates = []

        # Find all positions where first line matches
        for i, line in enumerate(content_lines):
            if line.strip() == first_line:
                # Check if last line matches at expected position
                expected_last = i + len(search_lines) - 1
                if expected_last < len(content_lines):
                    if content_lines[expected_last].strip() == last_line:
                        # Calculate similarity of middle content
                        block = '\n'.join(content_lines[i:expected_last+1])
                        similarity = self.levenshtein_similarity(block, search)

                        if similarity >= 0.3:  # Lower threshold for multi-candidate
                            candidates.append((i, expected_last, similarity))

        if len(candidates) == 1:
            # Single match - use very lenient threshold (0.0)
            i, last, _ = candidates[0]
            return self.replace_block(content_lines, i, last, replace)
        elif len(candidates) > 1:
            # Multiple matches - use best match above 0.3 threshold
            best = max(candidates, key=lambda x: x[2])
            if best[2] >= 0.3:
                return self.replace_block(content_lines, best[0], best[1], replace)

        return None

    def fuzzy_match(self, content: str, search: str, replace: str, threshold: float = 0.8) -> Optional[str]:
        """Strategy 2.4: Levenshtein distance-based matching"""
        search_lines = search.splitlines()
        content_lines = content.splitlines()

        best_match = None
        best_similarity = 0.0

        # Sliding window
        for i in range(len(content_lines) - len(search_lines) + 1):
            block = '\n'.join(content_lines[i:i+len(search_lines)])
            similarity = self.levenshtein_similarity(block, search)

            if similarity > best_similarity:
                best_similarity = similarity
                best_match = i

        if best_similarity >= threshold:
            # Found good match
            new_content = (
                content_lines[:best_match] +
                replace.splitlines() +
                content_lines[best_match+len(search_lines):]
            )
            return '\n'.join(new_content)

        return None

    def context_aware_match(self, content: str, search: str, replace: str) -> Optional[str]:
        """Strategy 2.5: Use surrounding context for matching"""
        # Extract context hints from search block
        context = self.extract_context_hints(search)

        # Find similar blocks with context matching
        candidates = self.find_blocks_with_context(content, search, context)

        if len(candidates) == 1:
            return self.apply_replacement(content, candidates[0], replace)
        elif len(candidates) > 1:
            # Use additional heuristics
            best = self.rank_candidates(candidates, context)
            return self.apply_replacement(content, best, replace)

        return None

    def dotdotdot_match(self, content: str, search: str, replace: str) -> Optional[str]:
        """Strategy 2.6: Handle ... for elided code"""
        if '...' not in search:
            return None

        # Split search into parts around ...
        parts = search.split('...')

        # Find block that matches all parts in sequence
        content_lines = content.splitlines()

        for i in range(len(content_lines)):
            positions = []
            current_pos = i

            for part in parts:
                # Find next occurrence of this part
                match_pos = self.find_part(content_lines, part, current_pos)
                if match_pos is None:
                    break
                positions.append(match_pos)
                current_pos = match_pos + len(part.splitlines())

            if len(positions) == len(parts):
                # All parts matched
                start = positions[0]
                end = current_pos
                return self.replace_block(content_lines, start, end, replace)

        return None

    def suggest_similar(self, content: str, search: str) -> str:
        """Find similar content to suggest to user"""
        content_lines = content.splitlines()
        search_lines = search.splitlines()

        # Find lines with high similarity
        suggestions = []
        for i, line in enumerate(content_lines):
            for search_line in search_lines:
                similarity = self.line_similarity(line, search_line)
                if similarity > 0.6:
                    suggestions.append((i+1, line, similarity))

        if suggestions:
            suggestions.sort(key=lambda x: x[2], reverse=True)
            result = "Did you mean:\n"
            for line_num, line, sim in suggestions[:5]:
                result += f"  Line {line_num}: {line} (similarity: {sim:.2f})\n"
            return result

        return "No similar lines found"

    def levenshtein_similarity(self, s1: str, s2: str) -> float:
        """Calculate similarity score (0-1) using Levenshtein distance"""
        distance = Levenshtein.distance(s1, s2)
        max_len = max(len(s1), len(s2))
        if max_len == 0:
            return 1.0
        return 1.0 - (distance / max_len)
```

#### Strategy 3: Unified Diff/Patch Application (OpenCode Approach)

```typescript
class PatchEditor {
  async applyPatch(filePath: string, patchText: string): Promise<EditResult> {
    try {
      // Parse unified diff
      const patch = parsePatch(patchText);

      // Read current file
      const content = await fs.readFile(filePath, 'utf-8');
      const lines = content.split('\n');

      // Apply each hunk
      for (const hunk of patch.hunks) {
        lines = await this.applyHunk(lines, hunk);
      }

      const newContent = lines.join('\n');
      await fs.writeFile(filePath, newContent);

      return {
        success: true,
        strategy: 'unified-diff',
        diff: createPatch(filePath, content, newContent)
      };
    } catch (error) {
      throw new StrategyFailed('unified-diff', error);
    }
  }

  private async applyHunk(lines: string[], hunk: Hunk): Promise<string[]> {
    // Find context match with fuzz factor
    const contextLines = hunk.lines.filter(l => l.type === 'context');
    const position = this.findBestMatch(lines, contextLines, hunk.oldStart);

    if (position === -1) {
      throw new Error('Cannot find context for hunk');
    }

    // Apply changes
    const result = [...lines];
    let offset = 0;

    for (const line of hunk.lines) {
      if (line.type === 'delete') {
        result.splice(position + offset, 1);
      } else if (line.type === 'insert') {
        result.splice(position + offset, 0, line.content);
        offset++;
      } else {
        offset++;
      }
    }

    return result;
  }

  private findBestMatch(lines: string[], contextLines: string[], hint: number): number {
    // Try exact position first
    if (this.matchesAtPosition(lines, contextLines, hint)) {
      return hint;
    }

    // Search nearby
    for (let offset = 1; offset <= 10; offset++) {
      if (this.matchesAtPosition(lines, contextLines, hint + offset)) {
        return hint + offset;
      }
      if (this.matchesAtPosition(lines, contextLines, hint - offset)) {
        return hint - offset;
      }
    }

    // Search entire file
    for (let i = 0; i < lines.length - contextLines.length; i++) {
      if (this.matchesAtPosition(lines, contextLines, i)) {
        return i;
      }
    }

    return -1;
  }
}
```

#### Strategy 4: Whole File Rewrite

```typescript
class WholeFileEditor {
  async rewrite(filePath: string, newContent: string): Promise<EditResult> {
    const oldContent = await fs.readFile(filePath, 'utf-8');

    // Generate diff for review
    const diff = createTwoFilesPatch(
      filePath,
      filePath,
      oldContent,
      newContent,
      'before',
      'after'
    );

    await fs.writeFile(filePath, newContent);

    return {
      success: true,
      strategy: 'whole-file-rewrite',
      diff,
      warning: 'Full file rewrite - review carefully'
    };
  }
}
```

### 3.3 Edit Orchestrator

```typescript
class EditOrchestrator {
  private strategies: EditStrategy[] = [
    new ToolBasedEditor(),
    new SearchReplaceEditor(),
    new PatchEditor(),
    new WholeFileEditor()
  ];

  async edit(request: EditRequest): Promise<EditResult> {
    const errors: Error[] = [];

    for (const strategy of this.strategies) {
      try {
        console.log(`Trying strategy: ${strategy.name}`);
        const result = await strategy.apply(request);

        if (result.success) {
          console.log(`✓ Success with ${strategy.name}`);
          return result;
        }
      } catch (error) {
        console.log(`✗ ${strategy.name} failed: ${error.message}`);
        errors.push(error);
      }
    }

    // All strategies failed
    throw new AllStrategiesFailedError(errors);
  }
}
```

---

## 4. Context Management (RepoMap)

### 4.1 Intelligent Codebase Understanding

**Key Innovation**: Use tree-sitter to parse 100+ languages and build dependency graphs.

**Implementation** (from Aider):

```python
class RepoMap:
    """Generate intelligent repository maps for LLM context"""

    def __init__(self, cache_dir: str = '.aider.tags.cache'):
        self.cache_dir = cache_dir
        self.languages = self.load_tree_sitter_languages()
        self.tag_cache = {}

    def get_repo_map(
        self,
        chat_files: List[str],
        other_files: List[str],
        mentioned_fnames: Set[str],
        mentioned_idents: Set[str]
    ) -> str:
        """
        Generate a repository map showing code structure

        Args:
            chat_files: Files currently in conversation
            other_files: Other relevant files in repo
            mentioned_fnames: Filenames mentioned by user/LLM
            mentioned_idents: Identifiers (classes, functions) mentioned

        Returns:
            Formatted repo map string for LLM context
        """

        # 1. Extract tags (classes, functions, methods) from all files
        all_tags = {}
        for file in chat_files + other_files:
            tags = self.get_tags(file)
            all_tags[file] = tags

        # 2. Build dependency graph
        graph = self.build_dependency_graph(all_tags)

        # 3. Rank files by relevance
        ranked = self.rank_files(
            graph,
            chat_files,
            mentioned_fnames,
            mentioned_idents
        )

        # 4. Generate map within token budget
        return self.generate_map(ranked, token_budget=8000)

    def get_tags(self, file_path: str) -> List[Tag]:
        """Extract code tags using tree-sitter"""

        # Check cache
        cache_key = self.get_cache_key(file_path)
        if cache_key in self.tag_cache:
            return self.tag_cache[cache_key]

        # Determine language
        language = self.detect_language(file_path)
        if language not in self.languages:
            return []  # Unsupported language

        # Parse with tree-sitter
        parser = Parser()
        parser.set_language(self.languages[language])

        code = read_file(file_path)
        tree = parser.parse(bytes(code, 'utf8'))

        # Run language-specific queries
        tags = []
        query = self.get_query_for_language(language)
        captures = query.captures(tree.root_node)

        for node, capture_name in captures:
            tag = Tag(
                name=self.get_identifier(node),
                kind=capture_name,  # 'class', 'function', 'method', etc.
                line=node.start_point[0] + 1,
                file=file_path
            )
            tags.append(tag)

        # Cache results
        self.tag_cache[cache_key] = tags
        return tags

    def get_query_for_language(self, language: str) -> Query:
        """Get tree-sitter query for extracting definitions"""

        queries = {
            'python': '''
                (class_definition name: (identifier) @class)
                (function_definition name: (identifier) @function)
            ''',
            'javascript': '''
                (class_declaration name: (identifier) @class)
                (function_declaration name: (identifier) @function)
                (method_definition name: (property_identifier) @method)
            ''',
            'typescript': '''
                (class_declaration name: (type_identifier) @class)
                (interface_declaration name: (type_identifier) @interface)
                (function_declaration name: (identifier) @function)
                (method_definition name: (property_identifier) @method)
            ''',
            'rust': '''
                (struct_item name: (type_identifier) @struct)
                (enum_item name: (type_identifier) @enum)
                (trait_item name: (type_identifier) @trait)
                (impl_item type: (_) @impl)
                (function_item name: (identifier) @function)
            ''',
            'go': '''
                (type_declaration (type_spec name: (type_identifier) @type))
                (function_declaration name: (identifier) @function)
                (method_declaration name: (field_identifier) @method)
            ''',
            # ... 100+ more languages
        }

        return Query(self.languages[language], queries[language])

    def build_dependency_graph(self, all_tags: Dict[str, List[Tag]]) -> nx.DiGraph:
        """Build dependency graph using networkx"""

        graph = nx.DiGraph()

        # Add nodes (one per file)
        for file in all_tags:
            graph.add_node(file)

        # Add edges (dependencies)
        for file, tags in all_tags.items():
            code = read_file(file)

            # Find references to other files' tags
            for other_file, other_tags in all_tags.items():
                if file == other_file:
                    continue

                for tag in other_tags:
                    # Check if this file references the tag
                    if self.has_reference(code, tag.name):
                        graph.add_edge(file, other_file, tag=tag.name)

        return graph

    def rank_files(
        self,
        graph: nx.DiGraph,
        chat_files: List[str],
        mentioned_fnames: Set[str],
        mentioned_idents: Set[str]
    ) -> List[Tuple[str, float]]:
        """Rank files by relevance using PageRank-style algorithm"""

        scores = {}

        # Base scores
        for file in graph.nodes():
            score = 0.0

            # Chat files are most important
            if file in chat_files:
                score += 10.0

            # Mentioned files
            if file in mentioned_fnames:
                score += 5.0

            # Files with mentioned identifiers
            tags = self.get_tags(file)
            for tag in tags:
                if tag.name in mentioned_idents:
                    score += 3.0

            scores[file] = score

        # PageRank-style propagation
        pagerank = nx.pagerank(graph, personalization=scores)

        # Combine scores
        final_scores = {}
        for file in graph.nodes():
            final_scores[file] = scores.get(file, 0) + pagerank[file] * 10

        # Sort by score
        ranked = sorted(final_scores.items(), key=lambda x: x[1], reverse=True)
        return ranked

    def generate_map(self, ranked_files: List[Tuple[str, float]], token_budget: int) -> str:
        """Generate formatted repo map within token budget"""

        lines = []
        tokens_used = 0

        for file, score in ranked_files:
            if tokens_used >= token_budget:
                break

            # File header
            header = f"\n{file}:\n"
            tokens_used += self.estimate_tokens(header)
            lines.append(header)

            # Tags for this file
            tags = self.get_tags(file)
            for tag in tags:
                line = f"  {tag.kind} {tag.name} (line {tag.line})\n"
                token_cost = self.estimate_tokens(line)

                if tokens_used + token_cost > token_budget:
                    break

                tokens_used += token_cost
                lines.append(line)

        return ''.join(lines)

    def estimate_tokens(self, text: str) -> int:
        """Estimate token count (rough approximation)"""
        return len(text) // 4
```

**Usage in LLM Context**:

```python
# Include repo map in system prompt
system_prompt = f"""You are an AI coding assistant.

Here is the repository structure:

{repo_map}

The user is working on: {', '.join(chat_files)}

Please help them with their request.
"""
```

**Benefits**:
- LLM understands codebase structure
- Discovers relevant files automatically
- Respects token limits
- Cached for performance
- Works with 100+ languages

---

## 5. Built-in LSP Integration

### 5.1 Language Server Protocol Support

**Key Innovation**: Immediate type checking and diagnostics after every edit (from OpenCode).

```typescript
class LSPManager {
  private servers: Map<string, LanguageServer> = new Map();
  private diagnostics: Map<string, Diagnostic[]> = new Map();

  async initialize() {
    // Auto-discover LSP configurations
    const config = await this.loadConfig();

    for (const [language, serverConfig] of Object.entries(config.lsp)) {
      await this.startServer(language, serverConfig);
    }
  }

  async startServer(language: string, config: LSPConfig) {
    const server = new LanguageServer({
      command: config.command,
      args: config.args,
      rootUri: this.workspaceRoot,
      capabilities: {
        textDocument: {
          hover: true,
          completion: true,
          definition: true,
          references: true,
          diagnostics: true
        }
      }
    });

    await server.start();

    // Subscribe to diagnostics
    server.on('textDocument/publishDiagnostics', (params) => {
      this.diagnostics.set(params.uri, params.diagnostics);
    });

    this.servers.set(language, server);
  }

  async touchFile(filePath: string, waitForDiagnostics: boolean = true) {
    const language = this.detectLanguage(filePath);
    const server = this.servers.get(language);

    if (!server) {
      return;  // No LSP for this language
    }

    // Notify LSP of file change
    const content = await fs.readFile(filePath, 'utf-8');
    await server.didChange({
      textDocument: {
        uri: `file://${filePath}`,
        version: Date.now()
      },
      contentChanges: [{
        text: content
      }]
    });

    if (waitForDiagnostics) {
      // Wait for diagnostics (up to 2 seconds)
      await this.waitForDiagnostics(filePath, 2000);
    }
  }

  async getDiagnostics(filePath?: string): Promise<Diagnostic[]> {
    if (filePath) {
      return this.diagnostics.get(`file://${filePath}`) || [];
    }

    // Return all diagnostics
    const all: Diagnostic[] = [];
    for (const diags of this.diagnostics.values()) {
      all.push(...diags);
    }
    return all;
  }

  async getHover(filePath: string, line: number, character: number): Promise<Hover> {
    const language = this.detectLanguage(filePath);
    const server = this.servers.get(language);

    if (!server) {
      return null;
    }

    return await server.hover({
      textDocument: { uri: `file://${filePath}` },
      position: { line, character }
    });
  }

  async getDefinition(filePath: string, line: number, character: number): Promise<Location[]> {
    const language = this.detectLanguage(filePath);
    const server = this.servers.get(language);

    if (!server) {
      return [];
    }

    return await server.definition({
      textDocument: { uri: `file://${filePath}` },
      position: { line, character }
    });
  }
}
```

**Configuration** (`opencode.json`):

```json
{
  "lsp": {
    "typescript": {
      "command": "typescript-language-server",
      "args": ["--stdio"],
      "rootPatterns": ["package.json", "tsconfig.json"]
    },
    "python": {
      "command": "pylsp",
      "args": [],
      "rootPatterns": ["setup.py", "pyproject.toml"]
    },
    "rust": {
      "command": "rust-analyzer",
      "args": [],
      "rootPatterns": ["Cargo.toml"]
    },
    "go": {
      "command": "gopls",
      "args": [],
      "rootPatterns": ["go.mod"]
    }
  }
}
```

**Integration with Post-Tool Validation**:

```typescript
// After every edit
await lsp.touchFile(filePath, true);
const diagnostics = await lsp.getDiagnostics(filePath);

if (diagnostics.some(d => d.severity === DiagnosticSeverity.Error)) {
  console.log('❌ LSP Errors detected:');
  for (const diag of diagnostics) {
    console.log(`  Line ${diag.range.start.line}: ${diag.message}`);
  }

  // Attempt auto-fix
  const fixed = await autoFix(filePath, diagnostics);
  if (!fixed) {
    throw new ValidationError('LSP errors could not be auto-fixed');
  }
}
```

---

## 6. Advanced Features

### 6.1 Confidence Scoring (Claude Code)

**Purpose**: Filter low-confidence code review feedback to reduce noise.

```typescript
class ConfidenceScorer {
  calculateConfidence(feedback: CodeReviewFeedback): number {
    let score = 0.0;

    // Factor 1: Specificity (0-30 points)
    if (feedback.includes('line')) score += 10;
    if (feedback.includes('function')) score += 10;
    if (/:\d+/.test(feedback)) score += 10;  // Line number reference

    // Factor 2: Actionability (0-30 points)
    const actionVerbs = ['change', 'add', 'remove', 'fix', 'refactor', 'rename'];
    for (const verb of actionVerbs) {
      if (feedback.toLowerCase().includes(verb)) {
        score += 10;
        break;
      }
    }
    if (feedback.includes('should') || feedback.includes('must')) score += 10;
    if (feedback.includes('```')) score += 10;  // Code example

    // Factor 3: Severity (0-40 points)
    if (feedback.toLowerCase().includes('security')) score += 20;
    if (feedback.toLowerCase().includes('bug')) score += 15;
    if (feedback.toLowerCase().includes('error')) score += 15;
    if (feedback.toLowerCase().includes('performance')) score += 10;

    return Math.min(score, 100) / 100;  // Normalize to 0-1
  }

  filterFeedback(feedback: CodeReviewFeedback[], threshold: number = 0.8): CodeReviewFeedback[] {
    return feedback.filter(item => {
      const confidence = this.calculateConfidence(item.message);
      item.confidence = confidence;
      return confidence >= threshold;
    });
  }
}
```

**Usage**:

```typescript
// In code review agent
const feedback = await this.generateCodeReview(files);
const filtered = this.confidenceScorer.filterFeedback(feedback, 0.8);

console.log(`Generated ${feedback.length} items, ${filtered.length} above threshold`);
return filtered;
```

### 6.2 Plan Mode (OpenCode)

**Purpose**: Safe exploration and analysis without execution.

```typescript
class PlanMode {
  private enabled: boolean = false;
  private allowedTools: Set<string> = new Set([
    'read', 'grep', 'glob', 'lsp', 'git_status', 'git_diff', 'git_log'
  ]);

  enable() {
    this.enabled = true;
    console.log('📋 Plan mode enabled - read-only operations only');
  }

  disable() {
    this.enabled = false;
    console.log('✏️  Plan mode disabled - full operations enabled');
  }

  async checkToolAllowed(toolName: string): Promise<void> {
    if (!this.enabled) {
      return;  // Plan mode not active
    }

    if (!this.allowedTools.has(toolName)) {
      throw new PlanModeError(
        `Tool '${toolName}' not allowed in plan mode. ` +
        `Only read-only operations permitted: ${Array.from(this.allowedTools).join(', ')}`
      );
    }
  }
}
```

**User Experience**:

```bash
$ code-assistant --plan
📋 Plan mode enabled

> Add user authentication with JWT tokens

I'll analyze your codebase and create a plan for implementing JWT authentication:

1. Reading current authentication setup...
   ✓ Found auth.ts with basic authentication
   ✓ No JWT implementation detected

2. Analyzing dependencies...
   ✓ Found jsonwebtoken in package.json
   ✓ No security middleware detected

3. Plan:
   Phase 1: Install dependencies
   - Add jsonwebtoken
   - Add bcrypt for password hashing

   Phase 2: Implement JWT service
   - Create src/services/jwt.service.ts
   - Generate/verify tokens
   - Refresh token mechanism

   Phase 3: Add authentication middleware
   - Create src/middleware/auth.middleware.ts
   - Protect routes

   Phase 4: Update user endpoints
   - POST /auth/login
   - POST /auth/register
   - POST /auth/refresh

   Phase 5: Testing
   - Unit tests for JWT service
   - Integration tests for auth flow

Ready to execute? [Y/n]
```

### 6.3 Multi-Agent Parallel Execution (Claude Code)

**Purpose**: Run multiple specialized agents concurrently for faster completion.

```typescript
class AgentOrchestrator {
  private agents: Map<string, Agent> = new Map();

  async executeParallel(tasks: Task[]): Promise<Map<string, any>> {
    // Group tasks by agent type
    const grouped = this.groupByAgent(tasks);

    // Launch agents in parallel
    const promises = [];
    for (const [agentType, agentTasks] of grouped.entries()) {
      const agent = this.getAgent(agentType);
      promises.push(
        this.executeAgent(agent, agentTasks)
      );
    }

    // Wait for all to complete
    const results = await Promise.allSettled(promises);

    // Aggregate results
    const aggregated = new Map();
    for (let i = 0; i < results.length; i++) {
      const result = results[i];
      const agentType = Array.from(grouped.keys())[i];

      if (result.status === 'fulfilled') {
        aggregated.set(agentType, result.value);
      } else {
        console.error(`Agent ${agentType} failed:`, result.reason);
        aggregated.set(agentType, { error: result.reason });
      }
    }

    return aggregated;
  }

  private async executeAgent(agent: Agent, tasks: Task[]): Promise<any> {
    // Create isolated context
    const context = agent.createContext();

    // Execute tasks
    const results = [];
    for (const task of tasks) {
      const result = await agent.execute(task, context);
      results.push(result);
    }

    return results;
  }
}
```

**Example Usage**:

```typescript
// User request: "Run tests, check linter, and build the project"

const tasks = [
  { type: 'test', agent: 'test-runner' },
  { type: 'lint', agent: 'linter' },
  { type: 'build', agent: 'builder' }
];

const results = await orchestrator.executeParallel(tasks);

console.log('✓ All tasks completed');
console.log('Tests:', results.get('test-runner'));
console.log('Lint:', results.get('linter'));
console.log('Build:', results.get('builder'));
```

### 6.4 Multi-Phase Workflows (Claude Code)

**Purpose**: Guide complex feature development through structured phases.

```typescript
class WorkflowEngine {
  private phases = [
    'discovery',
    'exploration',
    'questions',
    'architecture',
    'implementation',
    'review',
    'summary'
  ];

  async executeFeatureWorkflow(feature: FeatureRequest): Promise<FeatureResult> {
    const context = {
      feature,
      discoveries: [],
      explorations: [],
      answers: [],
      architecture: null,
      implementation: [],
      reviews: [],
      summary: null
    };

    for (const phase of this.phases) {
      console.log(`\n=== Phase: ${phase} ===\n`);

      const phaseResult = await this.executePhase(phase, context);
      context[phase] = phaseResult;

      // Check if user wants to continue
      if (phase !== 'summary') {
        const shouldContinue = await this.askUserToContinue(phase, phaseResult);
        if (!shouldContinue) {
          console.log('Workflow paused. You can resume later.');
          return context;
        }
      }
    }

    return context;
  }

  private async executePhase(phase: string, context: any): Promise<any> {
    switch (phase) {
      case 'discovery':
        return await this.discoveryPhase(context);
      case 'exploration':
        return await this.explorationPhase(context);
      case 'questions':
        return await this.questionsPhase(context);
      case 'architecture':
        return await this.architecturePhase(context);
      case 'implementation':
        return await this.implementationPhase(context);
      case 'review':
        return await this.reviewPhase(context);
      case 'summary':
        return await this.summaryPhase(context);
    }
  }

  private async discoveryPhase(context: any): Promise<Discovery> {
    // Search codebase for related code
    const related = await this.repoMap.findRelated(context.feature.description);

    // Analyze existing patterns
    const patterns = await this.analyzePatterns(related);

    // Identify dependencies
    const deps = await this.analyzeDependencies(related);

    return { related, patterns, deps };
  }

  private async explorationPhase(context: any): Promise<Exploration> {
    // Read and understand related files
    const understanding = await this.exploreAgent.analyze(context.discovery.related);

    // Identify integration points
    const integrationPoints = this.findIntegrationPoints(understanding);

    return { understanding, integrationPoints };
  }

  private async questionsPhase(context: any): Promise<Answers> {
    // Generate clarifying questions
    const questions = this.generateQuestions(context);

    if (questions.length === 0) {
      return { questions: [], answers: [] };
    }

    // Ask user
    const answers = await this.askUser(questions);

    return { questions, answers };
  }

  private async architecturePhase(context: any): Promise<Architecture> {
    // Design the solution
    const design = await this.architectAgent.design({
      feature: context.feature,
      discoveries: context.discovery,
      explorations: context.exploration,
      answers: context.questions.answers
    });

    // Write ADR
    const adr = await this.writeADR(design);

    return { design, adr };
  }

  private async implementationPhase(context: any): Promise<Implementation[]> {
    // Break down into tasks
    const tasks = this.breakDownIntoTasks(context.architecture.design);

    // Implement each task
    const implementations = [];
    for (const task of tasks) {
      console.log(`\nImplementing: ${task.description}`);
      const impl = await this.developerAgent.implement(task, context);
      implementations.push(impl);

      // Run tests after each task
      await this.runTests(impl.files);
    }

    return implementations;
  }

  private async reviewPhase(context: any): Promise<Review[]> {
    // Review all implemented code
    const reviews = [];
    for (const impl of context.implementation) {
      const review = await this.reviewerAgent.review(impl.files);
      reviews.push(review);

      // Apply high-confidence feedback
      const filtered = this.confidenceScorer.filterFeedback(review.feedback, 0.8);
      if (filtered.length > 0) {
        await this.applyFeedback(impl.files, filtered);
      }
    }

    return reviews;
  }

  private async summaryPhase(context: any): Promise<Summary> {
    // Generate comprehensive summary
    return {
      feature: context.feature.description,
      filesModified: this.collectFiles(context.implementation),
      testsAdded: this.collectTests(context.implementation),
      reviewFindings: this.summarizeReviews(context.review),
      nextSteps: this.suggestNextSteps(context)
    };
  }
}
```

---

## 7. Error Recovery & Rollback

### 7.1 Git-Based Recovery (Aider Approach)

```python
class GitRecovery:
    """Auto-commit every change for easy rollback"""

    def __init__(self, repo_path: str):
        self.repo = git.Repo(repo_path)
        self.commit_stack = []

    def auto_commit(self, files: List[str], message: str, strategy: str):
        """Commit changes with detailed message"""

        # Stage specific files
        for file in files:
            self.repo.index.add([file])

        # Create detailed commit message
        full_message = f"""{message}

Strategy: {strategy}
Files: {', '.join(files)}
Timestamp: {datetime.now().isoformat()}

🤖 Generated with AI Code Assistant

Co-Authored-By: Claude <noreply@anthropic.com>
"""

        # Commit
        commit = self.repo.index.commit(full_message)
        self.commit_stack.append(commit)

        return commit

    def undo(self, steps: int = 1):
        """Undo last N commits"""
        if steps > len(self.commit_stack):
            raise ValueError(f"Cannot undo {steps} steps, only {len(self.commit_stack)} commits")

        # Get commit to reset to
        target = self.commit_stack[-(steps + 1)] if steps < len(self.commit_stack) else None

        if target:
            self.repo.head.reset(target, index=True, working_tree=True)
        else:
            # Reset to before any AI commits
            self.repo.head.reset('HEAD~' + str(steps), index=True, working_tree=True)

        # Remove from stack
        self.commit_stack = self.commit_stack[:-steps]

    def show_history(self, limit: int = 10):
        """Show recent AI commits"""
        commits = list(self.repo.iter_commits(max_count=limit))

        for i, commit in enumerate(commits):
            if '🤖' in commit.message:
                print(f"{i+1}. {commit.hexsha[:7]} - {commit.message.split('\\n')[0]}")
```

### 7.2 Snapshot System (OpenCode Approach)

```typescript
class SnapshotManager {
  private snapshots: Map<string, Snapshot> = new Map();
  private snapshotDir: string;

  async createSnapshot(sessionId: string, description: string): Promise<string> {
    const snapshot: Snapshot = {
      id: this.generateId(),
      sessionId,
      timestamp: Date.now(),
      description,
      files: await this.captureFiles()
    };

    // Save to disk
    await this.saveSnapshot(snapshot);
    this.snapshots.set(snapshot.id, snapshot);

    return snapshot.id;
  }

  async restoreSnapshot(snapshotId: string): Promise<void> {
    const snapshot = this.snapshots.get(snapshotId);
    if (!snapshot) {
      throw new Error(`Snapshot ${snapshotId} not found`);
    }

    // Restore all files
    for (const [filePath, content] of Object.entries(snapshot.files)) {
      await fs.writeFile(filePath, content);
    }

    console.log(`✓ Restored snapshot: ${snapshot.description}`);
  }

  async autoSnapshot(event: string): Promise<string> {
    return await this.createSnapshot('auto', `Auto-snapshot: ${event}`);
  }

  private async captureFiles(): Promise<Map<string, string>> {
    const files = new Map();

    // Capture all tracked files
    const tracked = await this.getTrackedFiles();
    for (const file of tracked) {
      const content = await fs.readFile(file, 'utf-8');
      files.set(file, content);
    }

    return files;
  }
}
```

### 7.3 Integrated Recovery System

```typescript
class RecoveryManager {
  constructor(
    private git: GitRecovery,
    private snapshots: SnapshotManager
  ) {}

  async executeWithRecovery<T>(
    operation: () => Promise<T>,
    description: string
  ): Promise<T> {
    // Create snapshot before operation
    const snapshotId = await this.snapshots.autoSnapshot(`Before: ${description}`);

    try {
      // Execute operation
      const result = await operation();

      // Auto-commit on success
      await this.git.auto_commit(
        this.getModifiedFiles(),
        description,
        'auto'
      );

      return result;
    } catch (error) {
      console.error(`❌ Operation failed: ${error.message}`);

      // Ask user what to do
      const choice = await this.askRecoveryChoice();

      switch (choice) {
        case 'snapshot':
          await this.snapshots.restoreSnapshot(snapshotId);
          break;
        case 'git':
          await this.git.undo(1);
          break;
        case 'retry':
          return await this.executeWithRecovery(operation, description);
        case 'continue':
          // Do nothing, keep failed state
          break;
      }

      throw error;
    }
  }

  private async askRecoveryChoice(): Promise<string> {
    // Show options to user
    const choices = [
      'snapshot: Restore to snapshot before operation',
      'git: Undo last git commit',
      'retry: Try the operation again',
      'continue: Keep current state and continue'
    ];

    return await promptUser('Recovery options:', choices);
  }
}
```

---

## 8. Permission & Security

### 8.1 Permission System

```typescript
interface PermissionConfig {
  edit: 'allow' | 'deny' | 'ask';
  bash: {
    [pattern: string]: 'allow' | 'deny' | 'ask';
  };
  webfetch: 'allow' | 'deny' | 'ask';
  git: {
    push: 'allow' | 'deny' | 'ask';
    force: 'deny';
  };
}

class PermissionManager {
  private config: PermissionConfig;

  async allows(tool: string, params: any): Promise<boolean> {
    const permission = this.getPermission(tool, params);

    switch (permission) {
      case 'allow':
        return true;

      case 'deny':
        throw new PermissionDenied(`Tool ${tool} is not allowed`);

      case 'ask':
        return await this.askUser(tool, params);
    }
  }

  private getPermission(tool: string, params: any): 'allow' | 'deny' | 'ask' {
    // Special handling for bash commands
    if (tool === 'bash') {
      return this.getBashPermission(params.command);
    }

    // Direct tool permissions
    return this.config[tool] || 'ask';
  }

  private getBashPermission(command: string): 'allow' | 'deny' | 'ask' {
    const patterns = this.config.bash || {};

    // Check each pattern
    for (const [pattern, permission] of Object.entries(patterns)) {
      if (this.matchesPattern(command, pattern)) {
        return permission;
      }
    }

    // Default to ask
    return 'ask';
  }

  private matchesPattern(command: string, pattern: string): boolean {
    // Convert glob pattern to regex
    const regex = new RegExp(
      '^' + pattern.replace(/\*/g, '.*').replace(/\?/g, '.') + '$'
    );
    return regex.test(command);
  }

  private async askUser(tool: string, params: any): Promise<boolean> {
    console.log(`\n🔐 Permission required:`);
    console.log(`Tool: ${tool}`);
    console.log(`Params: ${JSON.stringify(params, null, 2)}`);

    const response = await promptUser('Allow? [y/N]', ['y', 'n']);
    return response.toLowerCase() === 'y';
  }
}
```

**Example Configuration**:

```json
{
  "permissions": {
    "edit": "allow",
    "bash": {
      "git*": "allow",
      "npm install*": "allow",
      "npm run*": "allow",
      "rm -rf*": "ask",
      "sudo*": "deny",
      "curl*": "ask"
    },
    "webfetch": "ask",
    "git": {
      "push": "ask",
      "force": "deny"
    }
  }
}
```

### 8.2 Enhanced Security: Knowledge-Graph-Based Command Permissions (Terraphim Innovation)

**Key Innovation**: Repository-specific security using knowledge graphs with intelligent command matching via terraphim-automata.

#### 8.2.1 Architecture

Instead of simple pattern matching, use terraphim's knowledge graph to store allowed/blocked commands per repository, with automata-based fuzzy matching and synonym resolution.

```rust
// terraphim_rolegraph/src/repository_security.rs

pub struct RepositorySecurityGraph {
    allowed_commands: RoleGraph,     // Commands that run without asking
    blocked_commands: RoleGraph,     // Commands that are NEVER allowed
    ask_commands: RoleGraph,         // Commands requiring confirmation
    command_synonyms: Thesaurus,     // Command aliases/variations
    automata: TerraphimAutomata,    // Fast command matching (Aho-Corasick)
    fuzzy_matcher: FuzzyMatcher,     // Jaro-Winkler + Levenshtein
}

impl RepositorySecurityGraph {
    /// Validate command from LLM output using multi-strategy matching
    pub async fn validate_command(&self, llm_command: &str) -> CommandPermission {
        // 1. Exact match using Aho-Corasick (nanoseconds)
        if let Some(exact) = self.automata.find_matches(llm_command, false) {
            return self.check_permission(exact);
        }

        // 2. Synonym resolution via thesaurus
        let normalized = self.normalize_command(llm_command);
        if let Some(known) = self.command_synonyms.find_synonym(&normalized) {
            println!("Resolved '{}' → '{}'", llm_command, known);
            return self.check_permission(known);
        }

        // 3. Fuzzy match with Jaro-Winkler (similarity ≥ 0.85)
        if let Some(fuzzy) = self.fuzzy_matcher.find_similar(llm_command, 0.85) {
            return self.check_permission(fuzzy);
        }

        // 4. Unknown command - default to ASK for safety
        CommandPermission::Ask(llm_command.to_string())
    }
}
```

#### 8.2.2 Repository Security Configuration

Each repository has `.terraphim/security.json`:

```json
{
  "repository": "my-rust-project",
  "security_level": "development",

  "allowed_commands": {
    "git": ["status", "diff", "log", "add", "commit", "branch"],
    "cargo": ["build", "test", "check", "clippy", "fmt", "doc"],
    "cat": ["*"],
    "ls": ["*"],
    "grep": ["*"],
    "find": ["*"]
  },

  "blocked_commands": {
    "git": ["push --force", "reset --hard", "clean -fd"],
    "cargo": ["publish", "yank"],
    "rm": ["-rf /", "-rf /*", "-rf ~"],
    "sudo": ["*"],
    "chmod": ["777 *"]
  },

  "ask_commands": {
    "git": ["push", "pull", "merge", "rebase"],
    "rm": ["*"],
    "mv": ["*"],
    "docker": ["*"]
  },

  "command_synonyms": {
    "delete file": "rm",
    "remove file": "rm",
    "erase": "rm",
    "show file": "cat",
    "display": "cat",
    "list files": "ls",
    "directory": "ls",
    "search": "grep",
    "find text": "grep",
    "build project": "cargo build",
    "run tests": "cargo test",
    "format code": "cargo fmt"
  },

  "contextual_permissions": [
    {
      "command": "cargo publish",
      "allowed_if": [
        {"branch_is": "main"},
        {"file_exists": "Cargo.toml"},
        {"file_contains": ["Cargo.toml", "version = "]}
      ]
    },
    {
      "command": "git push",
      "blocked_if": [
        {"branch_is": "main"},
        {"file_modified": [".env", "secrets.json"]}
      ]
    }
  ]
}
```

#### 8.2.3 Command Extraction from LLM Output

```rust
// terraphim_automata/src/command_matcher.rs

pub struct CommandMatcher {
    automata: AhoCorasickAutomata,
    extraction_patterns: Vec<Pattern>,
}

impl CommandMatcher {
    /// Extract commands from natural language LLM output
    pub fn extract_commands(&self, llm_output: &str) -> Vec<String> {
        let mut commands = Vec::new();

        // Pattern 1: Backticks - `cargo build`
        commands.extend(self.extract_backtick_commands(llm_output));

        // Pattern 2: Code blocks - ```bash\ncargo build\n```
        commands.extend(self.extract_code_blocks(llm_output));

        // Pattern 3: Shell prompts - $ cargo build
        commands.extend(self.extract_shell_prompts(llm_output));

        // Pattern 4: Action phrases - "Let me run cargo build"
        commands.extend(self.extract_action_phrases(llm_output));

        // Use automata for fast extraction
        self.automata.find_all_patterns(llm_output, &commands)
    }

    fn extract_action_phrases(&self, text: &str) -> Vec<String> {
        // Extract commands from natural language
        // "Let me run X", "I'll execute Y", "Running Z"
        let action_patterns = vec![
            r"(?i)(?:let me |I'll |I will )?(?:run|execute|call) (.+)",
            r"(?i)Running (.+)",
            r"(?i)Executing (.+)",
        ];

        // Use regex + automata for efficient extraction
        self.extract_with_patterns(text, &action_patterns)
    }
}
```

#### 8.2.4 Secure Command Execution

```rust
// terraphim_mcp_server/src/secure_executor.rs

pub struct SecureCommandExecutor {
    security_graph: RepositorySecurityGraph,
    command_matcher: CommandMatcher,
    audit_log: AuditLog,
    learning_system: SecurityLearner,
}

impl SecureCommandExecutor {
    pub async fn execute_from_llm(&self, llm_output: &str) -> Result<ExecutionResult> {
        // 1. Extract all commands from LLM output
        let commands = self.command_matcher.extract_commands(llm_output);

        let mut results = Vec::new();

        for cmd in commands {
            // 2. Match command using automata + fuzzy + synonyms
            let matched = self.command_matcher.match_command(&cmd);

            // 3. Check permission from knowledge graph
            let permission = self.security_graph.validate_command(&cmd).await?;

            // 4. Execute based on permission
            let result = match permission {
                CommandPermission::Allow => {
                    // Execute silently (no user interruption)
                    self.audit_log.log_allowed(&cmd);
                    self.execute_command(&cmd).await?
                },

                CommandPermission::Block => {
                    // Never execute, log for security review
                    self.audit_log.log_blocked(&cmd);
                    ExecutionResult::Blocked(format!("🚫 Blocked: {}", cmd))
                },

                CommandPermission::Ask(command) => {
                    // Ask user, learn from decision
                    println!("🔐 Permission required for: {}", command);

                    if self.ask_user_permission(&command).await? {
                        self.audit_log.log_approved(&command);

                        // Learn from approval
                        self.learning_system.record_decision(&command, true).await;

                        self.execute_command(&command).await?
                    } else {
                        self.audit_log.log_denied(&command);

                        // Learn from denial
                        self.learning_system.record_decision(&command, false).await;

                        ExecutionResult::Denied(command)
                    }
                }
            };

            results.push(result);
        }

        Ok(ExecutionResult::Multiple(results))
    }
}
```

#### 8.2.5 Learning System

The system learns from user decisions to reduce future prompts:

```rust
// terraphim_rolegraph/src/security_learning.rs

pub struct SecurityLearner {
    graph: RepositorySecurityGraph,
    decisions: VecDeque<UserDecision>,
    learning_threshold: usize,
}

impl SecurityLearner {
    pub async fn record_decision(&mut self, command: &str, allowed: bool) {
        self.decisions.push_back(UserDecision {
            command: command.to_string(),
            allowed,
            timestamp: Utc::now(),
            similarity_group: self.find_similar_commands(command),
        });

        // Analyze patterns after N decisions
        if self.decisions.len() >= self.learning_threshold {
            self.analyze_and_learn().await;
        }
    }

    async fn analyze_and_learn(&mut self) {
        // Group similar commands
        let command_groups = self.group_by_similarity(&self.decisions);

        for (group, decisions) in command_groups {
            let allowed_count = decisions.iter().filter(|d| d.allowed).count();
            let denied_count = decisions.len() - allowed_count;

            // Consistent approval → add to allowed list
            if allowed_count > 5 && denied_count == 0 {
                self.graph.add_allowed_command(group).await;
                println!("📝 Learned: '{}' is now auto-allowed", group);
            }

            // Consistent denial → add to blocked list
            else if denied_count > 3 && allowed_count == 0 {
                self.graph.add_blocked_command(group).await;
                println!("🚫 Learned: '{}' is now auto-blocked", group);
            }
        }

        // Persist updated graph
        self.graph.save().await?;
    }
}
```

#### 8.2.6 Context-Aware Permissions

Advanced feature: permissions depend on repository state:

```rust
pub enum PermissionCondition {
    BranchIs(String),              // Only on specific branch
    FileExists(String),            // Requires file to exist
    FileContains(String, String),  // File must contain pattern
    FileModified(Vec<String>),     // Block if files changed
    TimeWindow(TimeRange),         // Only during certain hours
    CommitCount(usize),            // After N commits
}

impl RepositorySecurityGraph {
    pub async fn check_contextual_permission(
        &self,
        command: &str,
        repo: &Repository,
    ) -> Result<bool> {
        let rules = self.contextual_rules.get(command);

        for rule in rules {
            // Check all conditions
            for condition in &rule.allowed_if {
                if !self.check_condition(condition, repo).await? {
                    return Ok(false);
                }
            }

            for condition in &rule.blocked_if {
                if self.check_condition(condition, repo).await? {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }
}
```

#### 8.2.7 Auto-Generated Security Profiles

System generates smart defaults based on repository type:

```rust
// terraphim_service/src/security_profiler.rs

pub async fn generate_security_profile(repo_path: &Path) -> SecurityConfig {
    let mut config = SecurityConfig::default();

    // Detect repository type
    let repo_type = detect_repo_type(repo_path).await;

    match repo_type {
        RepoType::Rust => {
            config.allowed_commands.insert("cargo", vec![
                "build", "test", "check", "clippy", "fmt", "doc"
            ]);
            config.blocked_commands.insert("cargo", vec![
                "publish", "yank"
            ]);
            config.command_synonyms.insert("build", "cargo build");
            config.command_synonyms.insert("test", "cargo test");
        },

        RepoType::JavaScript => {
            config.allowed_commands.insert("npm", vec![
                "install", "test", "run build", "run dev", "run lint"
            ]);
            config.blocked_commands.insert("npm", vec![
                "publish", "unpublish"
            ]);
        },

        RepoType::Python => {
            config.allowed_commands.insert("python", vec![
                "*.py", "test", "-m pytest", "-m unittest"
            ]);
            config.allowed_commands.insert("pip", vec![
                "install -r requirements.txt", "list", "show"
            ]);
        },

        _ => {}
    }

    // Always add safe operations
    config.allowed_commands.insert("cat", vec!["*"]);
    config.allowed_commands.insert("ls", vec!["*"]);
    config.allowed_commands.insert("grep", vec!["*"]);
    config.allowed_commands.insert("git", vec!["status", "diff", "log"]);

    // Always block dangerous operations
    config.blocked_commands.insert("rm", vec!["-rf /", "-rf /*"]);
    config.blocked_commands.insert("sudo", vec!["*"]);

    config
}
```

#### 8.2.8 Performance Characteristics

**Command Validation Speed**:
- Exact match (Aho-Corasick): ~10 nanoseconds
- Synonym lookup: ~100 nanoseconds
- Fuzzy match (Jaro-Winkler): ~1-5 microseconds
- Total overhead: < 10 microseconds per command

**Compared to Other Assistants**:

| Feature | Aider | Claude Code | OpenCode | Terraphim |
|---------|-------|-------------|----------|-----------|
| Command Permissions | ❌ None | ✅ Basic patterns | ✅ Basic | ✅ **Knowledge Graph** |
| Repository-Specific | ❌ | ❌ | ❌ | ✅ |
| Synonym Resolution | ❌ | ❌ | ❌ | ✅ |
| Fuzzy Command Matching | ❌ | ❌ | ❌ | ✅ |
| Learning System | ❌ | ❌ | ❌ | ✅ |
| Context-Aware | ❌ | Partial | ❌ | ✅ |
| Validation Speed | N/A | ~100µs | ~100µs | **~10µs** |

#### 8.2.9 Security Audit Trail

```rust
pub struct SecurityAuditLog {
    log_file: PathBuf,
    events: Vec<SecurityEvent>,
}

pub struct SecurityEvent {
    timestamp: DateTime<Utc>,
    command: String,
    matched_as: String,      // What the command matched in graph
    permission: CommandPermission,
    executed: bool,
    user_decision: Option<bool>,
    similarity_score: f64,
}

impl SecurityAuditLog {
    pub async fn log_event(&mut self, event: SecurityEvent) {
        self.events.push(event.clone());

        // Write to file for security review
        let entry = format!(
            "[{}] {} | Matched: {} | Permission: {:?} | Executed: {} | Similarity: {:.2}\n",
            event.timestamp,
            event.command,
            event.matched_as,
            event.permission,
            event.executed,
            event.similarity_score
        );

        fs::append(self.log_file, entry).await?;
    }

    pub fn generate_security_report(&self) -> SecurityReport {
        SecurityReport {
            total_commands: self.events.len(),
            allowed_auto: self.events.iter().filter(|e| matches!(e.permission, CommandPermission::Allow)).count(),
            blocked: self.events.iter().filter(|e| matches!(e.permission, CommandPermission::Block)).count(),
            asked: self.events.iter().filter(|e| matches!(e.permission, CommandPermission::Ask(_))).count(),
            learned_commands: self.count_learned_patterns(),
        }
    }
}
```

**Key Advantages of This Security Model**:

1. **Minimal Interruptions**: Known safe commands run automatically
2. **Repository-Specific**: Each project has its own security profile
3. **Intelligent Matching**: Handles command variations via fuzzy match + synonyms
4. **Learning System**: Reduces prompts over time by learning from user decisions
5. **Lightning Fast**: Aho-Corasick automata provides nanosecond exact matching
6. **Context-Aware**: Permissions can depend on branch, files, time, etc.
7. **Audit Trail**: Complete security log for compliance/review

This security model makes Terraphim the **safest code assistant** while being the **least intrusive**.

---

## 9. Testing & Quality Assurance

### 9.1 Testing Requirements

**Mandatory Rules**:
1. ❌ **No mocks in tests** (from Aider and OpenCode)
2. ✅ **Integration tests over unit tests** for file operations
3. ✅ **Benchmark-driven development** (from Aider)
4. ✅ **Coverage tracking** with minimum thresholds

```typescript
class TestRunner {
  async runTests(files: string[]): Promise<TestResult> {
    // 1. Run affected tests
    const tests = await this.findAffectedTests(files);

    console.log(`Running ${tests.length} affected tests...`);
    const result = await this.execute(tests);

    // 2. Check coverage
    if (this.config.coverageEnabled) {
      const coverage = await this.calculateCoverage(files);

      if (coverage < this.config.minCoverage) {
        throw new InsufficientCoverageError(
          `Coverage ${coverage}% is below minimum ${this.config.minCoverage}%`
        );
      }
    }

    return result;
  }

  async runBenchmarks(): Promise<BenchmarkResult> {
    // Run performance benchmarks
    const benchmarks = await this.findBenchmarks();

    const results = [];
    for (const benchmark of benchmarks) {
      console.log(`Running benchmark: ${benchmark.name}`);
      const result = await this.executeBenchmark(benchmark);
      results.push(result);

      // Check regression
      const baseline = await this.getBaseline(benchmark.name);
      if (result.duration > baseline * 1.1) {  // 10% regression threshold
        console.warn(`⚠️  Performance regression detected: ${benchmark.name}`);
      }
    }

    return { benchmarks: results };
  }
}
```

### 9.2 Benchmark-Driven Development (Aider Approach)

```python
class ExercismBenchmark:
    """Test against Exercism programming problems"""

    def run_benchmark(self, model: str) -> BenchmarkResult:
        problems = self.load_exercism_problems()

        results = {
            'passed': 0,
            'failed': 0,
            'errors': 0,
            'times': []
        }

        for problem in problems:
            start = time.time()

            try:
                # Have AI solve the problem
                solution = self.ai_solve(problem, model)

                # Run test suite
                test_result = self.run_problem_tests(problem, solution)

                if test_result.passed:
                    results['passed'] += 1
                else:
                    results['failed'] += 1

            except Exception as e:
                results['errors'] += 1
                print(f"Error on {problem.name}: {e}")

            duration = time.time() - start
            results['times'].append(duration)

        return results
```

---

## 10. Feature Comparison & Priorities

### 10.1 Complete Feature Matrix

| Feature | Claude Code | Aider | OpenCode | Required | Priority |
|---------|-------------|-------|----------|----------|----------|
| **Editing** |
| Tool-based edit | ✅ | ❌ | ✅ | ✅ | P0 |
| Text-based SEARCH/REPLACE | ❌ | ✅ | ❌ | ✅ | P0 |
| Unified diff/patch | ✅ | ✅ | ✅ | ✅ | P0 |
| Fuzzy matching | ❌ | ✅ (0.8) | ✅ (multiple) | ✅ | P0 |
| Levenshtein distance | ❌ | ✅ | ✅ | ✅ | P0 |
| Block anchor matching | ❌ | ❌ | ✅ | ✅ | P0 |
| Whitespace-flexible | ❌ | ✅ | ✅ | ✅ | P0 |
| Dotdotdot handling | ❌ | ✅ | ❌ | ✅ | P1 |
| Context-aware matching | ❌ | ❌ | ✅ | ✅ | P1 |
| Whole file rewrite | ✅ | ✅ | ✅ | ✅ | P2 |
| **Validation** |
| Pre-tool hooks | ✅ | ❌ | ❌ | ✅ | P0 |
| Post-tool hooks | ✅ | ❌ | ❌ | ✅ | P0 |
| Pre-LLM validation | ❌ | ❌ | ❌ | ✅ | P0 |
| Post-LLM validation | ❌ | ❌ | ❌ | ✅ | P0 |
| LSP integration | ✅ (via MCP) | ❌ | ✅ (built-in) | ✅ | P0 |
| Auto-linting | ✅ (via hooks) | ✅ | ❌ | ✅ | P0 |
| Test execution | ✅ (via hooks) | ✅ | ❌ | ✅ | P1 |
| Confidence scoring | ✅ (≥80) | ❌ | ❌ | ✅ | P1 |
| **Context** |
| RepoMap (tree-sitter) | ❌ | ✅ | ❌ | ✅ | P0 |
| Dependency analysis | ❌ | ✅ (networkx) | ❌ | ✅ | P1 |
| Token management | ✅ | ✅ | ✅ | ✅ | P0 |
| Cache system | ✅ | ✅ (disk) | ✅ (memory) | ✅ | P1 |
| 100+ languages | ✅ (via MCP) | ✅ | Limited | ✅ | P1 |
| **Architecture** |
| Plugin system | ✅ | Limited | ✅ | ✅ | P0 |
| Agent system | ✅ | Single | ✅ | ✅ | P0 |
| Parallel execution | ✅ | ❌ | ❌ | ✅ | P1 |
| Event hooks | ✅ (9 types) | ❌ | Limited | ✅ | P0 |
| Client/server | ❌ | ❌ | ✅ | ✅ | P1 |
| Permission system | ✅ | .aiderignore | ✅ | ✅ | P0 |
| **Recovery** |
| Git auto-commit | ✅ | ✅ | ❌ | ✅ | P0 |
| Undo command | ❌ | ✅ | ❌ | ✅ | P1 |
| Snapshot system | ❌ | ❌ | ✅ | ✅ | P1 |
| Rollback on error | ✅ | ✅ | ✅ | ✅ | P0 |
| **User Experience** |
| Plan mode | ✅ | ❌ | ✅ | ✅ | P1 |
| Extended thinking | ✅ | ❌ | ❌ | ✅ | P2 |
| Multi-phase workflows | ✅ | ❌ | ❌ | ✅ | P2 |
| CLI | ✅ | ✅ | ✅ | ✅ | P0 |
| TUI | ❌ | ❌ | ✅ | Optional | P2 |
| Web UI | ❌ | ❌ | Possible | Optional | P3 |
| **Integration** |
| GitHub (gh CLI) | ✅ | ❌ | ❌ | ✅ | P1 |
| MCP support | ✅ | ❌ | ❌ | ✅ | P1 |
| Multi-provider LLM | ✅ | ✅ (200+) | ✅ | ✅ | P0 |
| Local models | ✅ | ✅ | ✅ | ✅ | P1 |

**Priority Levels**:
- **P0**: Critical - Must have for MVP
- **P1**: Important - Include in v1.0
- **P2**: Nice to have - Include in v1.1+
- **P3**: Optional - Future consideration

---

## 11. Implementation Roadmap

### Phase 1: Core Foundation (Weeks 1-2)
**Goal**: Basic file editing with validation

- [ ] Project setup and architecture
- [ ] Tool-based editor (Strategy 1)
- [ ] Text-based SEARCH/REPLACE parser (Strategy 2.1-2.3)
- [ ] Pre-tool validation hooks
- [ ] Post-tool validation hooks
- [ ] Permission system (basic)
- [ ] Git auto-commit
- [ ] CLI interface

**Deliverable**: Can apply edits using tools OR text-based fallback with basic validation

### Phase 2: Advanced Editing (Weeks 3-4)
**Goal**: Robust multi-strategy editing

- [ ] Levenshtein fuzzy matching (Strategy 2.4)
- [ ] Context-aware matching (Strategy 2.5)
- [ ] Dotdotdot handling (Strategy 2.6)
- [ ] Unified diff/patch support (Strategy 3)
- [ ] Whole file rewrite (Strategy 4)
- [ ] Edit orchestrator with fallback chain
- [ ] Diff generation for all strategies

**Deliverable**: Highly reliable edit application with 9+ fallback strategies

### Phase 3: Validation Pipeline (Weeks 5-6)
**Goal**: 4-layer validation system

- [ ] Pre-LLM validation layer
- [ ] Post-LLM validation layer
- [ ] LSP manager (TypeScript, Python, Rust, Go)
- [ ] Auto-linter integration
- [ ] Test runner integration
- [ ] Confidence scoring system
- [ ] Error recovery with rollback

**Deliverable**: Complete validation pipeline catching errors at every stage

### Phase 4: Context Management (Weeks 7-8)
**Goal**: Intelligent codebase understanding

- [ ] Tree-sitter integration
- [ ] RepoMap implementation
- [ ] Language query definitions (20+ languages)
- [ ] Dependency graph builder (networkx)
- [ ] File ranking algorithm (PageRank-style)
- [ ] Token budget management
- [ ] Disk cache system

**Deliverable**: Automatic discovery of relevant code across codebase

### Phase 5: Agent System (Weeks 9-10)
**Goal**: Multi-agent orchestration

- [ ] Agent base class
- [ ] Specialized agents (developer, reviewer, debugger, etc.)
- [ ] Agent orchestrator
- [ ] Parallel execution engine
- [ ] Agent isolation (context, permissions)
- [ ] Inter-agent communication

**Deliverable**: Multiple specialized agents working in parallel

### Phase 6: Plugin Architecture (Weeks 11-12)
**Goal**: Extensibility and customization

- [ ] Plugin loader
- [ ] Hook system (9+ event types)
- [ ] Command registration
- [ ] Custom tool registration
- [ ] Plugin marketplace (design)
- [ ] Configuration system
- [ ] Plugin API documentation

**Deliverable**: Fully extensible system via plugins

### Phase 7: Advanced Features (Weeks 13-14)
**Goal**: Polish and advanced capabilities

- [ ] Plan mode
- [ ] Multi-phase workflows
- [ ] Snapshot system
- [ ] Extended thinking mode
- [ ] GitHub integration (gh CLI)
- [ ] MCP server/client
- [ ] Client/server architecture

**Deliverable**: Feature-complete system matching/exceeding existing tools

### Phase 8: Testing & Quality (Weeks 15-16)
**Goal**: Production-ready quality

- [ ] Integration test suite
- [ ] Benchmark suite (Exercism-style)
- [ ] Coverage tracking
- [ ] Performance profiling
- [ ] Security audit
- [ ] Documentation
- [ ] User guides

**Deliverable**: Production-ready v1.0 release

---

## 12. Technical Specifications

### 12.1 Tech Stack

**Language**: TypeScript + Rust (for performance-critical parts)

**Justification**:
- TypeScript: Rapid development, rich ecosystem, strong typing
- Rust: Performance-critical components (tree-sitter parsing, fuzzy matching)

**Core Libraries**:
```json
{
  "dependencies": {
    "tree-sitter": "^0.20.0",
    "tree-sitter-cli": "^0.20.0",
    "levenshtein-edit-distance": "^3.0.0",
    "diff": "^5.1.0",
    "diff-match-patch": "^1.0.5",
    "networkx": "via WASM or JS port",
    "anthropic-sdk": "^0.9.0",
    "openai": "^4.20.0",
    "hono": "^3.11.0",
    "ws": "^8.14.0",
    "commander": "^11.1.0",
    "chalk": "^5.3.0",
    "ora": "^7.0.1",
    "simple-git": "^3.20.0"
  }
}
```

### 12.2 File Structure

```
code-assistant/
├── packages/
│   ├── core/
│   │   ├── src/
│   │   │   ├── edit/
│   │   │   │   ├── strategies/
│   │   │   │   │   ├── tool-based.ts
│   │   │   │   │   ├── search-replace.ts
│   │   │   │   │   ├── patch.ts
│   │   │   │   │   └── whole-file.ts
│   │   │   │   ├── orchestrator.ts
│   │   │   │   └── index.ts
│   │   │   ├── validation/
│   │   │   │   ├── pre-llm.ts
│   │   │   │   ├── post-llm.ts
│   │   │   │   ├── pre-tool.ts
│   │   │   │   ├── post-tool.ts
│   │   │   │   └── pipeline.ts
│   │   │   ├── context/
│   │   │   │   ├── repo-map.ts
│   │   │   │   ├── tree-sitter.ts
│   │   │   │   ├── dependency-graph.ts
│   │   │   │   └── token-manager.ts
│   │   │   ├── agent/
│   │   │   │   ├── base.ts
│   │   │   │   ├── developer.ts
│   │   │   │   ├── reviewer.ts
│   │   │   │   ├── debugger.ts
│   │   │   │   └── orchestrator.ts
│   │   │   ├── lsp/
│   │   │   │   ├── manager.ts
│   │   │   │   ├── server.ts
│   │   │   │   └── diagnostics.ts
│   │   │   ├── recovery/
│   │   │   │   ├── git.ts
│   │   │   │   ├── snapshot.ts
│   │   │   │   └── manager.ts
│   │   │   ├── permission/
│   │   │   │   ├── manager.ts
│   │   │   │   └── config.ts
│   │   │   └── plugin/
│   │   │       ├── loader.ts
│   │   │       ├── hook.ts
│   │   │       └── registry.ts
│   │   └── package.json
│   ├── server/
│   │   ├── src/
│   │   │   ├── api/
│   │   │   ├── session/
│   │   │   └── index.ts
│   │   └── package.json
│   ├── cli/
│   │   ├── src/
│   │   │   ├── commands/
│   │   │   ├── ui/
│   │   │   └── index.ts
│   │   └── package.json
│   └── fuzzy-matcher/  (Rust via WASM)
│       ├── src/
│       │   ├── lib.rs
│       │   ├── levenshtein.rs
│       │   └── block-anchor.rs
│       └── Cargo.toml
├── plugins/
│   ├── example-plugin/
│   └── ...
├── benchmarks/
│   ├── exercism/
│   └── performance/
├── tests/
│   ├── integration/
│   └── e2e/
└── docs/
    ├── api/
    ├── guides/
    └── architecture/
```

### 12.3 Configuration Schema

```typescript
interface CodeAssistantConfig {
  // LLM Providers
  llm: {
    provider: 'anthropic' | 'openai' | 'google' | 'local';
    model: string;
    apiKey?: string;
    baseUrl?: string;
    maxTokens?: number;
  };

  // Validation
  validation: {
    preLLM: boolean;
    postLLM: boolean;
    preTool: boolean;
    postTool: boolean;
    autoLint: boolean;
    autoTest: boolean;
    confidenceThreshold: number;  // 0-1
  };

  // Editing
  editing: {
    strategies: string[];  // Order to try strategies
    fuzzyThreshold: number;  // 0-1
    contextLines: number;  // Lines of context for matching
  };

  // Context Management
  context: {
    repoMapEnabled: boolean;
    maxTokens: number;
    cacheDir: string;
    languages: string[];
  };

  // LSP
  lsp: {
    [language: string]: {
      command: string;
      args: string[];
      rootPatterns: string[];
    };
  };

  // Permissions
  permissions: {
    edit: 'allow' | 'deny' | 'ask';
    bash: {
      [pattern: string]: 'allow' | 'deny' | 'ask';
    };
    webfetch: 'allow' | 'deny' | 'ask';
    git: {
      push: 'allow' | 'deny' | 'ask';
      force: 'allow' | 'deny' | 'ask';
    };
  };

  // Recovery
  recovery: {
    autoCommit: boolean;
    snapshotEnabled: boolean;
    snapshotDir: string;
  };

  // Agents
  agents: {
    [name: string]: {
      enabled: boolean;
      permissions: Partial<PermissionConfig>;
      prompt?: string;
    };
  };

  // Plugins
  plugins: string[];

  // Testing
  testing: {
    minCoverage: number;  // 0-100
    benchmarkEnabled: boolean;
  };
}
```

---

## 13. Success Criteria

The coding assistant will be considered superior when it achieves:

### 13.1 Reliability
- [ ] **95%+ edit success rate** on first attempt across diverse codebases
- [ ] **Zero data loss** - all changes recoverable via git or snapshots
- [ ] **100% validation coverage** - no unchecked tool execution

### 13.2 Performance
- [ ] **<2s latency** for simple edits (tool-based)
- [ ] **<5s latency** for fuzzy-matched edits
- [ ] **<10s latency** for RepoMap generation (cached)
- [ ] **Handle 1000+ file repositories** efficiently

### 13.3 Quality
- [ ] **≥90% test coverage** for core modules
- [ ] **Zero critical security vulnerabilities**
- [ ] **LSP errors caught before commit** (when LSP available)
- [ ] **Confidence-filtered feedback** reduces noise by 50%+

### 13.4 Usability
- [ ] **No manual file path specification** - auto-discover via RepoMap
- [ ] **One-command feature implementation** using multi-phase workflows
- [ ] **Undo in <1s** using git or snapshots
- [ ] **Clear error messages** with actionable suggestions

### 13.5 Extensibility
- [ ] **10+ built-in agents** for common tasks
- [ ] **Plugin system** enables community extensions
- [ ] **Hook system** allows custom validation/automation
- [ ] **MCP compatibility** for tool integration

---

## 14. Conclusion

This requirements document specifies a coding assistant that combines:

1. **Aider's Reliability**: Text-based editing with multiple fallback strategies, works without tool support
2. **OpenCode's Validation**: Built-in LSP integration, 9+ edit strategies, immediate feedback
3. **Claude Code's Intelligence**: Multi-agent orchestration, confidence scoring, event-driven hooks

**Key Innovations**:
- **4-layer validation** (pre-LLM, post-LLM, pre-tool, post-tool)
- **9+ edit strategies** with automatic fallback
- **RepoMap context management** using tree-sitter
- **Built-in LSP integration** for real-time diagnostics
- **Multi-agent parallel execution** for complex tasks
- **Git + snapshot dual recovery** system

**The result**: A coding assistant that is more reliable than Aider, more intelligent than Claude Code, and more validating than OpenCode, while remaining fully extensible through plugins and hooks.

---

**Next Steps**:
1. Review and approve this requirements document
2. Set up development environment
3. Begin Phase 1 implementation
4. Establish CI/CD pipeline for continuous testing
5. Create plugin API and documentation
6. Build benchmark suite for measuring progress

**Estimated Timeline**: 16 weeks to v1.0 production release
**Team Size**: 2-4 developers recommended
**Language**: TypeScript + Rust (WASM for performance-critical parts)
