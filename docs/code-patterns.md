# Code Patterns: Tauri vs GPUI Implementation

## Overview

This document provides detailed code examples and patterns for both Tauri and GPUI implementations, highlighting architectural differences, best practices, and implementation strategies.

---

## 1. Data Flow Patterns

### Tauri Data Flow: Frontend → Backend → Frontend

**Pattern**: Svelte Component → Tauri Invoke → Rust Command → Service Layer → Response → Svelte Store Update

```typescript
// 1. User interacts with Svelte component
<script lang="ts">
    async function sendMessage() {
        // 2. Prepare request data
        const requestBody = {
            role: currentRole,
            messages: [userMessage],
            context: contexts
        };

        // 3. Call Tauri command
        const response = await invoke('chat', {
            request: requestBody
        }) as ChatResponse;

        // 4. Update Svelte store
        messages.update(msgs => [...msgs, response.message]);
    }
</script>
```

**Backend Flow**:
```rust
// 1. Tauri command receives request
#[command]
pub async fn chat(request: ChatRequest) -> Result<ChatResponse, TerraphimTauriError> {
    // 2. Validate and process
    let role_config = validate_role(request.role)?;

    // 3. Call service layer
    let llm_client = create_llm_client(&role_config)?;
    let response = llm_client.chat_completion(
        request.messages,
        None
    ).await?;

    // 4. Return response
    Ok(ChatResponse {
        status: Status::Success,
        reply: response.content,
        model: response.model,
    })
}
```

**Characteristics**:
- ✅ Clear separation of concerns
- ✅ Explicit command mapping
- ❌ Serialization overhead at each step
- ❌ Additional latency from bridge communication

### GPUI Data Flow: Direct Rust Integration

**Pattern**: GPUI View → Direct Rust Call → Service Layer → Entity Update → View Re-render

```rust
// 1. User interaction in GPUI view
impl Render for ChatView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        button("Send")
            .on_click(|_, this, cx| {
                // 2. Direct Rust call
                this.send_message(this.input.clone(), cx);
            })
    }
}

// 3. Direct service call
impl ChatView {
    pub fn send_message(&mut self, text: String, cx: &mut Context<Self>) {
        // 4. Spawn async task
        cx.spawn(|this, cx| async move {
            // 5. Direct service integration
            let response = llm::chat_completion(
                messages,
                this.current_role.clone()
            ).await?;

            // 6. Update entity state
            this.update(|this, cx| {
                this.messages.push(response.message);
                cx.notify();
            });
        });
    }
}
```

**Characteristics**:
- ✅ Direct integration without bridge
- ✅ Type-safe throughout
- ✅ Minimal overhead
- ✅ Unified codebase

---

## 2. Component Communication Patterns

### Tauri: Event Dispatching Pattern

**Child Component (Modal)**:

```svelte
<script lang="ts">
    import { createEventDispatcher } from 'svelte';

    const dispatch = createEventDispatcher<{
        create: ContextItem;
        update: ContextItem;
        delete: string;
        close: void;
    }>();

    function handleSave() {
        if (mode === 'edit') {
            dispatch('update', contextItem);
        } else {
            dispatch('create', contextItem);
        }
    }

    function handleClose() {
        dispatch('close');
    }
</script>

<div class="modal">
    <button onclick={handleSave}>Save</button>
    <button onclick={handleClose}>Close</button>
</div>
```

**Parent Component**:

```svelte
<script lang="ts">
    function handleCreate(event: CustomEvent<ContextItem>) {
        // Handle create event
        contexts.update(list => [...list, event.detail]);
    }

    function handleUpdate(event: CustomEvent<ContextItem>) {
        // Handle update event
        contexts.update(list => list.map(item =>
            item.id === event.detail.id ? event.detail : item
        ));
    }

    function handleClose() {
        showModal = false;
    }
</script>

<ContextEditModal
    bind:active={showModal}
    mode={editMode}
    on:create={handleCreate}
    on:update={handleUpdate}
    on:close={handleClose}
/>
```

**Characteristics**:
- ✅ Simple and intuitive
- ✅ Built into Svelte framework
- ❌ Runtime type checking only
- ❌ Events can be hard to trace

### GPUI: EventEmitter Pattern with Subscriptions

**Child Component (Modal)**:

```rust
pub struct ContextEditModal {
    event_sender: mpsc::UnboundedSender<ContextEditModalEvent>,
}

#[derive(Clone, Debug)]
pub enum ContextEditModalEvent {
    Create(ContextItem),
    Update(ContextItem),
    Delete(String),
    Close,
}

// EventEmitter trait
pub trait EventEmitter<T> {
    fn emit(&self, event: T);
}

impl EventEmitter<ContextEditModalEvent> for Entity<ContextEditModal> {
    fn emit(&self, event: ContextEditModalEvent) {
        // Implementation handled by subscription system
    }
}

impl ContextEditModal {
    fn handle_save(&mut self) {
        let context_item = self.build_context_item();
        self.event_sender
            .send(ContextEditModalEvent::Create(context_item))
            .ok();
    }

    fn handle_close(&mut self) {
        self.event_sender
            .send(ContextEditModalEvent::Close)
            .ok();
    }
}
```

**Parent Component**:

```rust
impl ChatView {
    fn setup_modal_subscriptions(cx: &mut Context<Self>, modal: &Entity<ContextEditModal>) {
        // Subscribe to modal events
        let subscription = cx.subscribe(modal, move |this, _modal, event: &ContextEditModalEvent, cx| {
            match event {
                ContextEditModalEvent::Create(context_item) => {
                    // Type-safe event handling
                    this.add_context(context_item.clone(), cx);
                }
                ContextEditModalEvent::Update(context_item) => {
                    this.update_context(context_item.clone(), cx);
                }
                ContextEditModalEvent::Delete(context_id) => {
                    this.delete_context(context_id.clone(), cx);
                }
                ContextEditModalEvent::Close => {
                    this.show_context_modal = false;
                }
            }
        });

        this._subscriptions.push(subscription);
    }
}
```

**Characteristics**:
- ✅ Compile-time type safety
- ✅ Explicit event types
- ❌ More boilerplate
- ❌ Requires understanding of EventEmitter pattern

---

## 3. Async Operation Handling

### Tauri: Promise-Based Async/Await

**Pattern**: JavaScript Promises with async/await

```typescript
async function loadConversationContext(conversationId: string) {
    try {
        // 1. Set loading state
        loading.set(true);

        // 2. Make async call
        const result = await invoke('get_conversation', {
            conversationId,
        }) as GetConversationResponse;

        // 3. Handle response
        if (result.status === 'success') {
            contexts.set(result.conversation.global_context || []);
        } else {
            throw new Error(result.error || 'Failed to load context');
        }
    } catch (error) {
        // 4. Error handling
        console.error('Error loading context:', error);
        errorStore.set(error.message);
    } finally {
        // 5. Cleanup
        loading.set(false);
    }
}

async function search(query: string) {
    if (!query.trim()) {
        searchResults.set([]);
        return;
    }

    // Debounce search
    clearTimeout(searchTimeout);
    searchTimeout = setTimeout(async () => {
        try {
            searchLoading.set(true);

            const result = await invoke('search', {
                query,
            }) as SearchResponse;

            if (result.status === 'success') {
                searchResults.set(result.results || []);
            }
        } catch (error) {
            console.error('Search error:', error);
            searchResults.set([]);
        } finally {
            searchLoading.set(false);
        }
    }, 300); // 300ms debounce
}
```

**Characteristics**:
- ✅ Familiar JavaScript pattern
- ✅ Easy error handling with try-catch
- ✅ Built-in async/await syntax
- ❌ Serialization overhead for each call

### GPUI: Tokio-Based Async/Await

**Pattern**: Rust async/await with Tokio runtime

```rust
impl ChatView {
    pub fn load_conversation_context(&mut self, conversation_id: ConversationId, cx: &mut Context<Self>) {
        let context_manager = self.context_manager.clone();

        // Spawn async task
        cx.spawn(async move |this, cx| {
            // Async operation
            let context_items = {
                let manager = context_manager.lock().await;
                manager.get_context_items(&conversation_id).unwrap_or_default()
            };

            // Update UI on main thread
            this.update(cx, |this, cx| {
                this.context_items = context_items;
                cx.notify();
            });
        });
    }

    pub fn search(&mut self, query: String, cx: &mut Context<Self>) {
        if query.trim().is_empty() {
            self.results.clear();
            cx.notify();
            return;
        }

        self.loading = true;

        let config_state = self.config_state.clone();
        let role = self.current_role.clone();

        // Spawn async task with cancellation support
        let task = cx.spawn(async move |this, cx| {
            let results = if let Some(config) = config_state {
                let service = TerraphimService::new().await;
                service.search(&query, Some(&role)).await.unwrap_or_default()
            } else {
                Vec::new()
            };

            // Debounce implementation
            tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;

            this.update(cx, |this, cx| {
                this.results = results;
                this.loading = false;
                cx.notify();
            });
        });

        // Store task handle for cancellation
        self.search_task = Some(task);
    }

    pub fn cancel_search(&mut self) {
        if let Some(task) = self.search_task.take() {
            task.abort();
        }
    }
}
```

**Error Handling with Result Types**:

```rust
pub fn send_message(&mut self, text: String, cx: &mut Context<Self>) {
    if self.is_sending || text.trim().is_empty() {
        return;
    }

    self.is_sending = true;
    let input = text.clone();

    let context_manager = self.context_manager.clone();
    let role = self.current_role.clone();

    cx.spawn(async move |this, cx| {
        // Result-based error handling
        match llm::chat_completion(
            vec![json!({ "role": "user", "content": input })],
            role,
        ).await {
            Ok(response) => {
                this.update(cx, |this, cx| {
                    this.messages.push(response.message);
                    this.is_sending = false;
                    cx.notify();
                });
            }
            Err(ServiceError::RateLimited { retry_after }) => {
                this.update(cx, |this, cx| {
                    this.messages.push(ChatMessage::system(format!(
                        "Rate limited. Retry after {} seconds.",
                        retry_after
                    )));
                    this.is_sending = false;
                    cx.notify();
                });
            }
            Err(e) => {
                log::error!("LLM error: {}", e);
                this.update(cx, |this, cx| {
                    this.messages.push(ChatMessage::system(format!(
                        "Error: {}",
                        e
                    )));
                    this.is_sending = false;
                    cx.notify();
                });
            }
        }
    });
}
```

**Characteristics**:
- ✅ Type-safe error handling with Result
- ✅ Zero-cost abstractions
- ✅ Cancellation support built-in
- ❌ More complex async patterns

---

## 4. State Management Patterns

### Tauri: Store-Based Reactive State

**Multiple Svelte Stores**:

```typescript
// stores.ts - Multiple specialized stores
export const configStore = writable<Config>(defaultConfig);
export const role = derived(configStore, $config => $config.selected_role);
export const persistentConversations = writable<ConversationSummary[]>([]);
export const contexts = writable<ContextItem[]>([]);
export const searchResults = writable<SearchResult[]>([]);

// Store composition and derived state
export const currentConversation = derived(
    [persistentConversations, currentPersistentConversationId],
    ([$conversations, $id]) => $conversations.find(c => c.id === $id)
);

export const totalContextItems = derived(
    contexts,
    $contexts => $contexts.length
);

// Computed values
export const canAddMoreContext = derived(
    [totalContextItems],
    $total => $total < 50
);

// Store updates
export function addContext(context: ContextItem) {
    contexts.update(list => [...list, context]);

    // Auto-save to backend
    if ($currentConversation) {
        invoke('add_context_to_conversation', {
            conversationId: $currentConversation.id,
            contextData: context,
        });
    }
}
```

**Component Usage**:

```svelte
<script lang="ts">
    // Subscribe to stores
    let contexts = $contexts;
    let role = $role;
    let canAddMore = $canAddMoreContext;

    // Derived state
    $: totalItems = contexts.length;
    $: isOverLimit = totalItems > 50;

    // Reactive updates
    $effect(() => {
        if (isOverLimit) {
            console.warn('Context limit exceeded');
        }
    });

    async function addContext() {
        // Update store
        contexts.update(list => [...list, newContext]);

        // Store auto-saves via subscription
    }
</script>

<div>
    <p>Total items: {totalItems}</p>
    <p class:warning={isOverLimit}>
        {canAddMore ? 'Can add more' : 'Limit reached'}
    </p>
</div>
```

**Characteristics**:
- ✅ Reactive and automatic
- ✅ Derived state built-in
- ✅ Multiple stores for separation
- ❌ Runtime type checking only

### GPUI: Entity-Based State Management

**Single Entity with Structured State**:

```rust
pub struct ChatView {
    // All state in one entity
    context_manager: Arc<TokioMutex<TerraphimContextManager>>,
    config_state: Option<ConfigState>,
    current_conversation_id: Option<ConversationId>,
    current_role: RoleName,
    messages: Vec<ChatMessage>,
    context_items: Vec<ContextItem>,
    input: String,
    is_sending: bool,
    show_context_panel: bool,
    _subscriptions: Vec<Subscription>,
}

impl ChatView {
    pub fn add_context(&mut self, context_item: ContextItem, cx: &mut Context<Self>) {
        // Update entity state
        self.context_items.push(context_item);

        // Trigger re-render
        cx.notify();

        // Async backend operation
        let manager = self.context_manager.clone();
        if let Some(conv_id) = &self.current_conversation_id {
            cx.spawn(async move |_this, _cx| {
                let mut mgr = manager.lock().await;
                mgr.add_context(conv_id, context_item).await.unwrap();
            });
        }
    }
}
```

**Derived State with Computed Properties**:

```rust
impl ChatView {
    // Computed property
    pub fn total_context_items(&self) -> usize {
        self.context_items.len()
    }

    pub fn can_add_more_context(&self) -> bool {
        self.total_context_items() < 50
    }

    pub fn is_over_limit(&self) -> bool {
        self.total_context_items() > 50
    }

    pub fn has_active_conversation(&self) -> bool {
        self.current_conversation_id.is_some()
    }
}
```

**Parent-Child State Sharing**:

```rust
pub struct App {
    // Parent owns shared state
    config_state: ConfigState,
    search_view: Entity<SearchView>,
    chat_view: Entity<ChatView>,
}

impl App {
    fn share_state_with_views(&mut self, cx: &mut Context<Self>) {
        // Pass ConfigState to children
        self.search_view.update(cx, |view, cx| {
            view.with_config(self.config_state.clone());
        });

        self.chat_view.update(cx, |view, cx| {
            view.with_config(self.config_state.clone());
        });
    }
}
```

**Characteristics**:
- ✅ Compile-time type safety
- ✅ Explicit state ownership
- ✅ No hidden reactive behavior
- ❌ More manual state management

---

## 5. Error Handling Strategies

### Tauri: Try-Catch with Result Serialization

**Frontend Error Handling**:

```typescript
async function sendMessage() {
    try {
        const result = await invoke('chat', {
            request: requestBody
        }) as ChatResponse;

        if (result.status === 'error') {
            throw new Error(result.error || 'Unknown error');
        }

        // Success handling
        addMessage(result.message);
    } catch (error: any) {
        // Categorize errors
        if (error.message.includes('Network')) {
            showNotification('Network error. Please check your connection.', 'error');
        } else if (error.message.includes('Rate limit')) {
            showNotification('Too many requests. Please wait.', 'warning');
        } else if (error.message.includes('Authentication')) {
            showNotification('Authentication failed. Please re-login.', 'error');
        } else {
            showNotification(`Error: ${error.message}`, 'error');
        }

        // Log for debugging
        console.error('Chat error:', {
            error,
            request: requestBody,
            timestamp: new Date().toISOString(),
        });
    }
}

// Global error handler
window.addEventListener('unhandledrejection', (event) => {
    console.error('Unhandled promise rejection:', event.reason);
    showNotification('An unexpected error occurred', 'error');
});
```

**Backend Error Handling**:

```rust
#[command]
pub async fn chat(request: ChatRequest) -> Result<ChatResponse, TerraphimTauriError> {
    // Convert domain errors to Tauri errors
    let result = llm_service.chat(request)
        .await
        .map_err(|e| match e {
            ServiceError::RateLimited { retry_after } => {
                TerraphimTauriError::RateLimited {
                    message: format!("Rate limited. Retry after {} seconds.", retry_after),
                    retry_after,
                }
            }
            ServiceError::AuthenticationFailed => {
                TerraphimTauriError::AuthenticationFailed {
                    message: "Authentication failed".to_string(),
                }
            }
            ServiceError::NetworkError(_) => {
                TerraphimTauriError::NetworkError {
                    message: "Network error occurred".to_string(),
                }
            }
            other => TerraphimTauriError::Other {
                message: other.to_string(),
            }
        })?;

    Ok(ChatResponse {
        status: Status::Success,
        reply: result.content,
        model: result.model,
    })
}

// Error enum for serialization
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
pub enum TerraphimTauriError {
    #[serde(rename = "rate_limited")]
    RateLimited {
        message: String,
        retry_after: u64,
    },
    #[serde(rename = "authentication_failed")]
    AuthenticationFailed {
        message: String,
    },
    #[serde(rename = "network_error")]
    NetworkError {
        message: String,
    },
    #[serde(rename = "other")]
    Other {
        message: String,
    },
}
```

**Characteristics**:
- ✅ Familiar try-catch syntax
- ✅ Easy to understand
- ❌ Runtime errors only
- ❌ Serialization of error types

### GPUI: Result Types with Pattern Matching

**Frontend Error Handling**:

```rust
impl ChatView {
    pub fn send_message(&mut self, text: String, cx: &mut Context<Self>) {
        let input = text.clone();

        cx.spawn(async move |this, cx| {
            // Result-based error handling
            match llm::chat_completion(
                vec![json!({ "role": "user", "content": input })],
                this.current_role.clone(),
            ).await {
                Ok(response) => {
                    // Success - update UI
                    this.update(cx, |this, cx| {
                        this.messages.push(response.message);
                        this.is_sending = false;
                        cx.notify();
                    });
                }
                Err(LlmError::RateLimited { retry_after }) => {
                    // Handle rate limiting
                    this.update(cx, |this, cx| {
                        this.messages.push(ChatMessage::system(format!(
                            "Rate limited. Retry after {} seconds.",
                            retry_after
                        )));
                        this.is_sending = false;
                        cx.notify();
                    });
                }
                Err(LlmError::AuthenticationFailed) => {
                    // Handle auth error
                    log::error!("LLM authentication failed");
                    this.update(cx, |this, cx| {
                        this.messages.push(ChatMessage::system(
                            "Authentication failed. Please check your API key.".to_string()
                        ));
                        this.is_sending = false;
                    });
                }
                Err(LlmError::NetworkError(e)) => {
                    // Handle network error
                    log::error!("Network error: {}", e);
                    this.update(cx, |this, cx| {
                        this.messages.push(ChatMessage::system(
                            "Network error. Please check your connection.".to_string()
                        ));
                        this.is_sending = false;
                    });
                }
                Err(e) => {
                    // Handle other errors
                    log::error!("Unexpected LLM error: {}", e);
                    this.update(cx, |this, cx| {
                        this.messages.push(ChatMessage::system(format!(
                            "Error: {}",
                            e
                        )));
                        this.is_sending = false;
                    });
                }
            }
        });
    }
}
```

**Custom Error Types**:

```rust
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("Conversation not found: {id}")]
    ConversationNotFound { id: ConversationId },

    #[error("Context item not found: {id}")]
    ContextItemNotFound { id: String },

    #[error("Rate limited: retry after {retry_after} seconds")]
    RateLimited { retry_after: u64 },

    #[error("Authentication failed")]
    AuthenticationFailed,

    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    #[error("LLM error: {0}")]
    LlmError(#[from] LlmError),

    #[error("Other error: {0}")]
    Other(String),
}

// Result alias
pub type ServiceResult<T> = Result<T, ServiceError>;

// Context manager with error handling
impl TerraphimContextManager {
    pub async fn get_conversation(
        &self,
        conversation_id: &ConversationId,
    ) -> ServiceResult<Arc<Conversation>> {
        self.conversations_cache
            .get(conversation_id)
            .cloned()
            .ok_or_else(|| ServiceError::ConversationNotFound {
                id: conversation_id.clone(),
            })
    }

    pub async fn add_context_item(
        &mut self,
        conversation_id: &ConversationId,
        context_data: ContextItemData,
    ) -> ServiceResult<AddContextResult> {
        // Check if conversation exists
        let conversation = self.conversations_cache
            .get(conversation_id)
            .ok_or_else(|| ServiceError::ConversationNotFound {
                id: conversation_id.clone(),
            })?;

        // Validate context data
        if context_data.title.trim().is_empty() {
            return Err(ServiceError::Other(
                "Title cannot be empty".to_string()
            ));
        }

        if context_data.content.trim().is_empty() {
            return Err(ServiceError::Other(
                "Content cannot be empty".to_string()
            ));
        }

        // Proceed with adding context
        let context_item = ContextItem::from(context_data);
        // ... implementation
        Ok(AddContextResult { warning: None })
    }
}
```

**Characteristics**:
- ✅ Compile-time error checking
- ✅ Exhaustive pattern matching
- ✅ No serialization overhead
- ❌ More verbose error handling

---

## 6. Configuration Management Patterns

### Tauri: Store-Based with $effect

**Configuration Store with Persistence**:

```typescript
// configStore.ts
export const configStore = writable<Config>(loadConfigFromStorage());

function loadConfigFromStorage(): Config {
    try {
        const saved = localStorage.getItem('terraphim-config');
        if (saved) {
            return JSON.parse(saved);
        }
    } catch (error) {
        console.error('Error loading config from storage:', error);
    }
    return defaultConfig;
}

function saveConfigToStorage(config: Config) {
    try {
        localStorage.setItem('terraphim-config', JSON.stringify(config));
    } catch (error) {
        console.error('Error saving config to storage:', error);
    }
}

// Auto-save on changes
$effect(() => {
    const config = $configStore;
    saveConfigToStorage(config);
});

// Update role with persistence
export async function selectRole(roleName: string) {
    configStore.update(config => ({
        ...config,
        selected_role: roleName
    }));

    // Update backend
    if ($is_tauri) {
        try {
            await invoke('select_role', { role: roleName });
        } catch (error) {
            console.error('Error selecting role:', error);
        }
    }

    // Trigger role-specific updates
    await refreshRoleData();
}

// Load configuration from backend
export async function loadConfig() {
    if ($is_tauri) {
        try {
            const result = await invoke('get_config') as GetConfigResponse;
            if (result.status === 'success') {
                configStore.set(result.config);
            }
        } catch (error) {
            console.error('Error loading config:', error);
        }
    }
}

// Reactive configuration updates
$effect(() => {
    const config = $configStore;

    // Update theme
    document.documentElement.setAttribute('data-theme', config.theme);

    // Update API endpoints
    API_BASE_URL = config.server_url || 'http://localhost:3000';

    // Update role-specific settings
    if (config.roles) {
        // Refresh autocomplete
        refreshAutocomplete();
    }
});
```

**Component Usage**:

```svelte
<script lang="ts">
    let config = $configStore;
    let selectedRole = $role;

    // Reactive updates
    $effect(() => {
        console.log('Role changed to:', selectedRole);
        loadRoleData(selectedRole);
    });

    async function handleRoleChange(event: Event) {
        const target = event.target as HTMLSelectElement;
        await selectRole(target.value);
    }

    // Subscribe to config changes
    $effect(() => {
        if (config.roles) {
            // Update UI based on config
            updateUIForConfig(config);
        }
    });
</script>

<div>
    <select bind:value={selectedRole} onchange={handleRoleChange}>
        {#each Object.keys(config.roles || {}) as roleName}
            <option value={roleName}>{roleName}</option>
        {/each}
    </select>

    <p>Server: {config.server_url}</p>
    <p>Theme: {config.theme}</p>
</div>
```

**Characteristics**:
- ✅ Auto-persistence to localStorage
- ✅ Reactive updates with $effect
- ✅ Derived state built-in
- ❌ Runtime type checking only

### GPUI: ConfigState with Arc<Mutex<Config>>

**Centralized ConfigState**:

```rust
pub struct ConfigState {
    pub config: Arc<Mutex<Config>>,
    pub roles: AHashMap<RoleName, RoleGraphSync>,
}

impl ConfigState {
    pub fn new(cx: &Context) -> Self {
        let config = Arc::new(Mutex::new(Self::load_config()));

        Self {
            config,
            roles: AHashMap::new(),
        }
    }

    fn load_config() -> Config {
        // Load from file or use default
        // Implementation for reading config file
        Config::default()
    }

    pub async fn get_selected_role(&self) -> RoleName {
        let config = self.config.lock().await;
        config.selected_role.clone()
    }

    pub async fn update_role(&self, role_name: RoleName) -> Result<(), ServiceError> {
        let mut config = self.config.lock().await;
        config.selected_role = role_name.clone();

        // Save to file
        self.save_config(&config)?;

        Ok(())
    }

    pub async fn update_server_url(&self, url: String) -> Result<(), ServiceError> {
        let mut config = self.config.lock().await;
        config.server_url = Some(url);

        // Save to file
        self.save_config(&config)?;

        Ok(())
    }

    fn save_config(&self, config: &Config) -> Result<(), ServiceError> {
        // Save to file
        let json = serde_json::to_string(config)
            .map_err(|e| ServiceError::Other(e.to_string()))?;

        std::fs::write("config.json", json)
            .map_err(|e| ServiceError::Other(e.to_string()))?;

        Ok(())
    }
}

// App with ConfigState
pub struct App {
    config_state: ConfigState,
    search_view: Entity<SearchView>,
    chat_view: Entity<ChatView>,
}

impl App {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let config_state = ConfigState::new(cx);

        Self {
            config_state,
            search_view: cx.new(|cx| {
                SearchView::new(window, cx)
            }),
            chat_view: cx.new(|cx| {
                ChatView::new(window, cx)
            }),
        }
    }

    fn setup_config_subscriptions(&mut self, cx: &mut Context<Self>) {
        // Pass config to child views
        self.search_view.update(cx, |view, cx| {
            view.with_config(self.config_state.clone());
        });

        self.chat_view.update(cx, |view, cx| {
            view.with_config(self.config_state.clone());
        });
    }
}
```

**Role Management with ConfigState**:

```rust
impl RoleSelector {
    pub fn change_role(&mut self, role: RoleName, cx: &mut Context<Self>) {
        let config_state = self.config_state.clone();

        cx.spawn(async move |this, cx| {
            // Update role in config
            if let Err(e) = config_state.update_role(role.clone()).await {
                log::error!("Failed to update role: {}", e);
                return;
            }

            // Update UI
            this.update(|this, cx| {
                this.selected_role = role;
                this.is_open = false;
                cx.notify();
            });

            // Trigger refresh
            cx.notify();
        });
    }

    pub fn get_current_role(&self) -> impl Future<Output = RoleName> + '_ {
        self.config_state.get_selected_role()
    }
}
```

**Characteristics**:
- ✅ Compile-time type safety
- ✅ Explicit async operations
- ✅ File-based persistence
- ❌ More manual state management

---

## 7. Testing Approaches

### Tauri: Vitest + Playwright

**Unit Test with Vitest**:

```typescript
// Chat.test.ts
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent, waitFor } from '@testing-library/svelte';
import Chat from '$lib/Chat/Chat.svelte';
import { invoke } from '@tauri-apps/api/tauri';

vi.mock('@tauri-apps/api/tauri', () => ({
    invoke: vi.fn(),
}));

describe('Chat Component', () => {
    beforeEach(() => {
        vi.clearAllMocks();
    });

    it('sends a message when Enter is pressed', async () => {
        const mockInvoke = vi.mocked(invoke);
        mockInvoke.mockResolvedValue({
            status: 'success',
            reply: 'Test response',
        });

        render(Chat);

        const input = screen.getByPlaceholderText(/type your message/i);
        const sendButton = screen.getByRole('button', { name: /send/i });

        // Type message
        await fireEvent.input(input, {
            target: { value: 'Test message' },
        });

        // Click send
        await fireEvent.click(sendButton);

        // Verify invoke was called
        await waitFor(() => {
            expect(mockInvoke).toHaveBeenCalledWith('chat', {
                request: expect.objectContaining({
                    messages: expect.arrayContaining([
                        expect.objectContaining({
                            role: 'user',
                            content: 'Test message',
                        }),
                    ]),
                }),
            });
        });

        // Verify response is displayed
        await waitFor(() => {
            expect(screen.getByText('Test response')).toBeVisible();
        });
    });

    it('handles errors gracefully', async () => {
        const mockInvoke = vi.mocked(invoke);
        mockInvoke.mockRejectedValue(new Error('Network error'));

        render(Chat);

        const input = screen.getByPlaceholderText(/type your message/i);
        const sendButton = screen.getByRole('button', { name: /send/i });

        await fireEvent.input(input, {
            target: { value: 'Test message' },
        });

        await fireEvent.click(sendButton);

        // Verify error is displayed
        await waitFor(() => {
            expect(screen.getByText(/error:/i)).toBeVisible();
        });
    });
});
```

**E2E Test with Playwright**:

```typescript
// chat.e2e.spec.ts
import { test, expect } from '@playwright/test';

test.describe('Chat E2E', () => {
    test.beforeEach(async ({ page }) => {
        await page.goto('/');
        await page.waitForLoadState('networkidle');
    });

    test('full chat workflow', async ({ page }) => {
        // Open chat
        await page.click('[data-testid="nav-chat"]');

        // Send message
        await page.fill('[data-testid="chat-input"]', 'Hello');
        await page.click('[data-testid="send-button"]');

        // Verify message appears
        await expect(page.locator('text=Hello')).toBeVisible();

        // Wait for response
        await expect(page.locator('text=Test response')).toBeVisible({
            timeout: 10000,
        });

        // Add context
        await page.click('[data-testid="add-context-button"]');
        await page.fill('[data-testid="context-title"]', 'Test Context');
        await page.fill('[data-testid="context-content"]', 'Test content');
        await page.click('[data-testid="save-context"]');

        // Verify context is added
        await expect(page.locator('text=Test Context')).toBeVisible();
    });
});
```

### GPUI: Tokio Tests + Integration Tests

**Unit Test with Tokio**:

```rust
// context_manager_test.rs
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_create_conversation() {
        let mut manager = TerraphimContextManager::new(ContextConfig::default());

        let conversation_id = manager
            .create_conversation("Test".to_string(), RoleName::from("Engineer"))
            .await
            .unwrap();

        assert!(manager
            .get_conversation(&conversation_id)
            .await
            .is_ok());
    }

    #[tokio::test]
    async fn test_add_context_with_limit() {
        let mut manager = TerraphimContextManager::new(ContextConfig {
            max_context_items: 2,
            ..Default::default()
        });

        let conversation_id = manager
            .create_conversation("Test".to_string(), RoleName::from("Engineer"))
            .await
            .unwrap();

        // Add first context - should succeed
        let result = manager
            .add_context_item(
                &conversation_id,
                ContextItemData {
                    title: "Context 1".to_string(),
                    content: "Content 1".to_string(),
                    context_type: ContextType::Document,
                    summary: None,
                    metadata: AHashMap::new(),
                },
            )
            .await
            .unwrap();

        assert!(result.warning.is_none());

        // Add second context - should succeed
        let result = manager
            .add_context_item(
                &conversation_id,
                ContextItemData {
                    title: "Context 2".to_string(),
                    content: "Content 2".to_string(),
                    context_type: ContextType::Document,
                    summary: None,
                    metadata: AHashMap::new(),
                },
            )
            .await
            .unwrap();

        assert!(result.warning.is_none());

        // Add third context - should get warning
        let result = manager
            .add_context_item(
                &conversation_id,
                ContextItemData {
                    title: "Context 3".to_string(),
                    content: "Content 3".to_string(),
                    context_type: ContextType::Document,
                    summary: None,
                    metadata: AHashMap::new(),
                },
            )
            .await
            .unwrap();

        assert!(result.warning.is_some());
        assert!(result.warning.unwrap().contains("limit"));
    }

    #[tokio::test]
    async fn test_concurrent_access() {
        let mut manager = TerraphimContextManager::new(ContextConfig::default());

        let conversation_id = manager
            .create_conversation("Test".to_string(), RoleName::from("Engineer"))
            .await
            .unwrap();

        // Simulate concurrent access
        let mut handles = Vec::new();

        for i in 0..10 {
            let manager_clone = &mut manager;
            let conv_id = conversation_id.clone();

            let handle = tokio::spawn(async move {
                manager_clone
                    .add_context_item(
                        &conv_id,
                        ContextItemData {
                            title: format!("Context {}", i),
                            content: format!("Content {}", i),
                            context_type: ContextType::Document,
                            summary: None,
                            metadata: AHashMap::new(),
                        },
                    )
                    .await
            });

            handles.push(handle);
        }

        // Wait for all tasks
        for handle in handles {
            handle.await.unwrap().unwrap();
        }

        // Verify all contexts were added
        let conversation = manager.get_conversation(&conversation_id).await.unwrap();
        assert_eq!(conversation.global_context.len(), 10);
    }
}
```

**Integration Test**:

```rust
// chat_integration_test.rs
#[cfg(test)]
mod integration_tests {
    use super::*;
    use terraphim_desktop_gpui::views::chat::ChatView;

    #[tokio::test]
    async fn test_chat_with_context() {
        let (window, cx) = gpui::Window::new().split();

        // Create chat view
        let chat_view = cx.new(|cx| {
            ChatView::new(&mut window.clone(), cx)
        });

        // Add context
        chat_view.update(&mut cx, |chat, cx| {
            chat.add_context(
                ContextItem {
                    id: "test".to_string(),
                    context_type: ContextType::Document,
                    title: "Test Context".to_string(),
                    content: "Test content".to_string(),
                    summary: None,
                    metadata: AHashMap::new(),
                    created_at: chrono::Utc::now(),
                    relevance_score: None,
                },
                cx,
            );
        });

        // Send message
        chat_view.update(&mut cx, |chat, cx| {
            chat.send_message("Test message".to_string(), cx);
        });

        // Wait for response (in real test, would mock LLM)
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        // Verify state
        chat_view.update(&mut cx, |chat, _cx| {
            assert_eq!(chat.context_items.len(), 1);
            assert_eq!(chat.messages.len(), 2); // user + assistant
        });
    }
}
```

**Characteristics**:
- ✅ Tauri: Familiar testing stack (Vitest + Playwright)
- ✅ GPUI: Comprehensive async testing with Tokio
- ❌ Tauri: Requires mocking Tauri API
- ❌ GPUI: More complex test setup

---

## 8. Performance Optimization Patterns

### Tauri: Debouncing and Virtual Scrolling

**Debounced Search**:

```typescript
let searchTimeout: NodeJS.Timeout;

function debouncedSearch(query: string) {
    clearTimeout(searchTimeout);

    searchTimeout = setTimeout(async () => {
        searchLoading.set(true);

        try {
            const result = await invoke('search', { query }) as SearchResponse;
            if (result.status === 'success') {
                searchResults.set(result.results || []);
            }
        } catch (error) {
            console.error('Search error:', error);
        } finally {
            searchLoading.set(false);
        }
    }, 300); // 300ms debounce
}
```

**Virtual Scrolling for Large Lists**:

```svelte
<script lang="ts">
    export let items: any[] = [];
    export let itemHeight = 60;
    export let containerHeight = 400;

    let scrollTop = 0;
    let containerElement: HTMLElement;

    // Calculate visible range
    $: visibleStart = Math.floor(scrollTop / itemHeight);
    $: visibleEnd = Math.min(
        visibleStart + Math.ceil(containerHeight / itemHeight) + 5,
        items.length
    );
    $: visibleItems = items.slice(visibleStart, visibleEnd);
    $: offsetY = visibleStart * itemHeight;
</script>

<div
    bind:this={containerElement}
    style="height: {containerHeight}px; overflow-y: auto;"
    on:scroll={(e) => scrollTop = e.currentTarget.scrollTop}
>
    <div style="height: {items.length * itemHeight}px; position: relative;">
        <div style="transform: translateY({offsetY}px);">
            {#each visibleItems as item, i (item.id)}
                <div style="height: {itemHeight}px; border-bottom: 1px solid #eee;">
                    <!-- Render item -->
                    {item.title}
                </div>
            {/each}
        </div>
    </div>
</div>
```

### GPUI: Async Caching and Efficient Rendering

**LRU Cache for Performance**:

```rust
use lru::LruCache;

pub struct SearchState {
    // Cache for search results
    search_cache: LruCache<String, Vec<SearchResult>>,

    // Cache for autocomplete
    autocomplete_cache: LruCache<String, Vec<AutocompleteSuggestion>>,

    // Cache for rendered chunks
    render_cache: Arc<DashMap<String, RenderedChunk>>,
}

impl SearchState {
    pub fn search(&mut self, query: String, cx: &mut Context<Self>) {
        // Check cache first
        if let Some(cached_results) = self.search_cache.get(&query) {
            self.results = cached_results.clone();
            cx.notify();
            return;
        }

        self.loading = true;

        let config_state = self.config_state.clone();
        let query_clone = query.clone();

        cx.spawn(async move |this, cx| {
            let results = if let Some(config) = config_state {
                let service = TerraphimService::new().await;
                service.search(&query_clone).await.unwrap_or_default()
            } else {
                Vec::new()
            };

            // Cache results
            this.search_cache.put(query_clone, results.clone());

            this.update(cx, |this, cx| {
                this.results = results;
                this.loading = false;
                cx.notify();
            });
        });
    }
}
```

**Efficient Async Task Management**:

```rust
pub struct ChatView {
    active_tasks: Vec<Task<()>>,
    search_task: Option<Task<()>>,
}

impl ChatView {
    pub fn send_message(&mut self, text: String, cx: &mut Context<Self>) {
        // Cancel previous task if still running
        if let Some(task) = self.search_task.take() {
            task.abort();
        }

        self.is_sending = true;

        // Spawn new task
        let task = cx.spawn(async move |this, cx| {
            // Async operation
            let result = llm::chat_completion(messages, role).await;

            this.update(cx, |this, cx| {
                match result {
                    Ok(response) => {
                        this.messages.push(response.message);
                    }
                    Err(e) => {
                        log::error!("LLM error: {}", e);
                        this.messages.push(ChatMessage::error(e.to_string()));
                    }
                }
                this.is_sending = false;
                cx.notify();
            });
        });

        self.search_task = Some(task);
    }
}
```

**Virtual Scrolling with GPUI**:

```rust
pub struct VirtualScrollState {
    viewport_height: f32,
    item_height: f32,
    total_items: usize,
    scroll_offset: f32,
    visible_range: (usize, usize),
}

impl VirtualScrollState {
    pub fn get_visible_range(&self) -> (usize, usize) {
        let start = (self.scroll_offset / self.item_height).floor() as usize;
        let visible_count = (self.viewport_height / self.item_height).ceil() as usize;
        let end = (start + visible_count).min(self.total_items);
        (start, end)
    }
}

impl Render for ChatView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let visible_range = self.virtual_scroll.get_visible_range();

        // Only render visible items
        div()
            .flex()
            .flex_col()
            .h_full()
            .children(
                self.messages
                    .iter()
                    .enumerate()
                    .filter(|(idx, _)| *idx >= visible_range.0 && *idx <= visible_range.1)
                    .map(|(idx, message)| {
                        self.render_message(idx, message, window, cx)
                    })
            )
            .render(window, cx);
    }
}
```

**Characteristics**:
- ✅ Tauri: Debouncing for search, virtual scrolling for lists
- ✅ GPUI: LRU caches, efficient async tasks, GPU-accelerated rendering
- ❌ Tauri: Web-based performance limitations
- ❌ GPUI: More complex optimization patterns

---

## 9. Summary

### Pattern Comparison Matrix

| Pattern | Tauri | GPUI | Winner |
|---------|-------|------|--------|
| **Data Flow** | Invoke → Command → Response | Direct Call | 🏆 GPUI (no bridge) |
| **Component Communication** | Event Dispatching | EventEmitter | 🏆 GPUI (type-safe) |
| **Async Handling** | Promises | Tokio | 🏆 GPUI (zero-cost) |
| **State Management** | Svelte Stores | Entity-Component | 🤔 Depends on preference |
| **Error Handling** | Try-Catch | Result Types | 🏆 GPUI (compile-time) |
| **Configuration** | Store-based | ConfigState | 🤔 Depends on preference |
| **Testing** | Vitest + Playwright | Tokio Tests | 🤔 Depends on preference |
| **Performance** | Good | Excellent | 🏆 GPUI (2x faster) |

### Key Takeaways

1. **Tauri**: Familiar web patterns, rapid development, good ecosystem
2. **GPUI**: Type safety, performance, unified codebase, native feel
3. **Both**: Production-ready with comprehensive features
4. **Choice**: Depends on team skills and performance requirements

The GPUI implementation demonstrates superior architectural patterns for performance-critical desktop applications, while Tauri offers faster development iteration and web developer familiarity.
