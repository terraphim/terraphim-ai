use gpui::*;
use std::sync::Arc;
use ulid::Ulid;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;

use crate::components::{ReusableComponent, ComponentConfig, PerformanceTracker, ComponentError, ViewContext, LifecycleEvent, ServiceRegistry};
use terraphim_types::{Document, ContextItem, ContextType};

/// Configuration for add document modal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddDocumentModalConfig {
    /// Whether to enable file upload
    pub enable_file_upload: bool,
    /// Whether to enable URL import
    pub enable_url_import: bool,
    /// Whether to enable text input
    pub enable_text_input: bool,
    /// Whether to enable metadata editing
    pub enable_metadata_editing: bool,
    /// Supported file extensions
    pub supported_extensions: Vec<String>,
    /// Maximum file size (in bytes)
    pub max_file_size: usize,
    /// Theme colors
    pub theme: AddDocumentTheme,
}

impl Default for AddDocumentModalConfig {
    fn default() -> Self {
        Self {
            enable_file_upload: true,
            enable_url_import: true,
            enable_text_input: true,
            enable_metadata_editing: true,
            supported_extensions: vec![
                "txt".to_string(),
                "md".to_string(),
                "pdf".to_string(),
                "doc".to_string(),
                "docx".to_string(),
                "html".to_string(),
            ],
            max_file_size: 10 * 1024 * 1024, // 10MB
            theme: AddDocumentTheme::default(),
        }
    }
}

/// Theme configuration for add document modal
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddDocumentTheme {
    pub background: gpui::Rgba,
    pub border: gpui::Rgba,
    pub text_primary: gpui::Rgba,
    pub text_secondary: gpui::Rgba,
    pub accent: gpui::Rgba,
    pub hover: gpui::Rgba,
    pub success: gpui::Rgba,
    pub warning: gpui::Rgba,
    pub error: gpui::Rgba,
    pub input_bg: gpui::Rgba,
    pub button_primary: gpui::Rgba,
    pub button_secondary: gpui::Rgba,
}

impl Default for AddDocumentTheme {
    fn default() -> Self {
        Self {
            background: gpui::Rgba::white(),
            border: gpui::Rgba::from_rgb(0.85, 0.85, 0.85),
            text_primary: gpui::Rgba::from_rgb(0.1, 0.1, 0.1),
            text_secondary: gpui::Rgba::from_rgb(0.5, 0.5, 0.5),
            accent: gpui::Rgba::from_rgb(0.2, 0.5, 0.8),
            hover: gpui::Rgba::from_rgb(0.95, 0.95, 0.98),
            success: gpui::Rgba::from_rgb(0.2, 0.7, 0.2),
            warning: gpui::Rgba::from_rgb(0.8, 0.6, 0.0),
            error: gpui::Rgba::from_rgb(0.8, 0.2, 0.2),
            input_bg: gpui::Rgba::from_rgb(0.98, 0.98, 1.0),
            button_primary: gpui::Rgba::from_rgb(0.2, 0.5, 0.8),
            button_secondary: gpui::Rgba::from_rgb(0.7, 0.7, 0.7),
        }
    }
}

/// Input methods for adding documents
#[derive(Debug, Clone, PartialEq)]
pub enum DocumentInputMethod {
    FileUpload,
    UrlImport,
    TextInput,
    Clipboard,
}

/// Document processing state
#[derive(Debug, Clone, PartialEq)]
pub enum DocumentProcessingState {
    Idle,
    Uploading,
    Processing,
    Completed,
    Error(String),
}

/// State for add document modal
#[derive(Debug, Clone)]
pub struct AddDocumentModalState {
    /// Current input method
    input_method: DocumentInputMethod,
    /// Processing state
    processing_state: DocumentProcessingState,
    /// Form inputs
    title: String,
    content: String,
    url: String,
    summary: String,
    /// Metadata
    tags: Vec<String>,
    custom_metadata: ahash::AHashMap<String, String>,
    /// File upload state
    selected_file: Option<String>,
    file_content: Option<String>,
    /// Validation errors
    title_error: Option<String>,
    content_error: Option<String>,
    url_error: Option<String>,
    /// UI state
    is_open: bool,
    show_metadata: bool,
    show_preview: bool,
    /// Mount state
    is_mounted: bool,
    /// Performance metrics
    last_update: std::time::Instant,
    processing_start_time: Option<std::time::Instant>,
}

/// Events emitted by AddDocumentModal
#[derive(Debug, Clone)]
pub enum AddDocumentModalEvent {
    /// Document was successfully added to context
    DocumentAdded {
        document: Arc<Document>,
        context_item: Arc<ContextItem>,
    },
    /// Processing failed
    ProcessingFailed {
        error: String,
        input_method: DocumentInputMethod,
    },
    /// Modal was closed
    ModalClosed,
    /// Input method changed
    InputMethodChanged {
        method: DocumentInputMethod,
    },
}

/// Document processor trait for different input types
#[async_trait]
pub trait DocumentProcessor: Send + Sync {
    async fn process_document(&self, input: &str, metadata: Option<ahash::AHashMap<String, String>>) -> Result<String, String>;
    fn supported_extensions(&self) -> Vec<String>;
    fn name(&self) -> &str;
}

/// Text processor for plain text files
pub struct TextDocumentProcessor;

#[async_trait]
impl DocumentProcessor for TextDocumentProcessor {
    async fn process_document(&self, content: &str, _metadata: Option<ahash::AHashMap<String, String>>) -> Result<String, String> {
        if content.trim().is_empty() {
            return Err("Document content is empty".to_string());
        }
        Ok(content.to_string())
    }

    fn supported_extensions(&self) -> Vec<String> {
        vec!["txt".to_string(), "md".to_string(), "text".to_string()]
    }

    fn name(&self) -> &str {
        "Text Processor"
    }
}

/// Markdown processor with basic formatting
pub struct MarkdownDocumentProcessor;

#[async_trait]
impl DocumentProcessor for MarkdownDocumentProcessor {
    async fn process_document(&self, content: &str, _metadata: Option<ahash::AHashMap<String, String>>) -> Result<String, String> {
        if content.trim().is_empty() {
            return Err("Markdown content is empty".to_string());
        }

        // Basic markdown processing (remove excessive formatting for context)
        let processed = content
            .lines()
            .filter(|line| !line.trim().starts_with('#')) // Remove headers for cleaner context
            .collect::<Vec<_>>()
            .join("\n");

        Ok(processed)
    }

    fn supported_extensions(&self) -> Vec<String> {
        vec!["md".to_string(), "markdown".to_string()]
    }

    fn name(&self) -> &str {
        "Markdown Processor"
    }
}

/// URL fetcher for web content
pub struct UrlDocumentFetcher;

impl UrlDocumentFetcher {
    pub async fn fetch_url(&self, url: &str) -> Result<String, String> {
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Err("Invalid URL format".to_string());
        }

        // Use reqwest for HTTP requests (would need to be added to dependencies)
        // For now, simulate URL fetching
        match url {
            u if u.contains("example.com") => Ok("Example document content from URL".to_string()),
            u if u.contains("github.com") => Ok("GitHub repository content".to_string()),
            _ => Ok(format!("Fetched content from {}", url)),
        }
    }
}

/// Modal component for adding documents to context
pub struct AddDocumentModal {
    config: AddDocumentModalConfig,
    state: AddDocumentModalState,
    performance_tracker: PerformanceTracker,
    id: String,
    processors: Vec<Box<dyn DocumentProcessor>>,
    url_fetcher: UrlDocumentFetcher,
    event_emitter: Option<Box<dyn gpui::EventEmitter<AddDocumentModalEvent>>>,
}

impl AddDocumentModal {
    /// Create a new add document modal
    pub fn new(config: AddDocumentModalConfig) -> Self {
        let id = Ulid::new().to_string().to_string();

        let processors: Vec<Box<dyn DocumentProcessor>> = vec![
            Box::new(TextDocumentProcessor),
            Box::new(MarkdownDocumentProcessor),
        ];

        Self {
            config,
            state: AddDocumentModalState {
                input_method: DocumentInputMethod::TextInput,
                processing_state: DocumentProcessingState::Idle,
                title: String::new(),
                content: String::new(),
                url: String::new(),
                summary: String::new(),
                tags: Vec::new(),
                custom_metadata: ahash::AHashMap::new(),
                selected_file: None,
                file_content: None,
                title_error: None,
                content_error: None,
                url_error: None,
                is_open: false,
                show_metadata: false,
                show_preview: false,
                is_mounted: false,
                last_update: std::time::Instant::now(),
                processing_start_time: None,
            },
            performance_tracker: PerformanceTracker::new(id.clone()),
            id,
            processors,
            url_fetcher: UrlDocumentFetcher,
            event_emitter: None,
        }
    }

    /// Open the modal
    pub fn open(&mut self, cx: &mut gpui::Context<'_, Self>) {
        self.state.is_open = true;
        self.state.last_update = std::time::Instant::now();
        cx.notify();
    }

    /// Close the modal
    pub fn close(&mut self, cx: &mut gpui::Context<'_, Self>) {
        self.state.is_open = false;
        self.reset_form();

        // Emit close event
        self.event_emitter.update(cx, |emitter, cx| {
            emitter.emit(AddDocumentModalEvent::ModalClosed, cx);
        });

        cx.notify();
    }

    /// Set input method
    pub fn set_input_method(&mut self, method: DocumentInputMethod, cx: &mut gpui::Context<Self>) {
        self.state.input_method = method;
        self.reset_form();

        // Emit method change event
        self.event_emitter.update(cx, |emitter, cx| {
            emitter.emit(AddDocumentModalEvent::InputMethodChanged { method }, cx);
        });

        cx.notify();
    }

    /// Set title
    pub fn set_title(&mut self, title: String) {
        self.state.title = title;
        self.state.title_error = None;
        self.state.last_update = std::time::Instant::now();
    }

    /// Set content
    pub fn set_content(&mut self, content: String) {
        self.state.content = content;
        self.state.content_error = None;
        self.state.last_update = std::time::Instant::now();
    }

    /// Set URL
    pub fn set_url(&mut self, url: String) {
        self.state.url = url;
        self.state.url_error = None;
        self.state.last_update = std::time::Instant::now();
    }

    /// Set summary
    pub fn set_summary(&mut self, summary: String) {
        self.state.summary = summary;
        self.state.last_update = std::time::Instant::now();
    }

    /// Add tag
    pub fn add_tag(&mut self, tag: String) {
        let tag = tag.trim().to_lowercase();
        if !tag.is_empty() && !self.state.tags.contains(&tag) {
            self.state.tags.push(tag);
            self.state.last_update = std::time::Instant::now();
        }
    }

    /// Remove tag
    pub fn remove_tag(&mut self, index: usize) {
        if index < self.state.tags.len() {
            self.state.tags.remove(index);
            self.state.last_update = std::time::Instant::now();
        }
    }

    /// Set custom metadata
    pub fn set_metadata(&mut self, key: String, value: String) {
        if !key.trim().is_empty() {
            self.state.custom_metadata.insert(key.trim().to_string(), value);
            self.state.last_update = std::time::Instant::now();
        }
    }

    /// Toggle metadata panel
    pub fn toggle_metadata(&mut self) {
        self.state.show_metadata = !self.state.show_metadata;
        self.state.last_update = std::time::Instant::now();
    }

    /// Toggle preview
    pub fn toggle_preview(&mut self) {
        self.state.show_preview = !self.state.show_preview;
        self.state.last_update = std::time::Instant::now();
    }

    /// Process document based on current input method
    pub async fn process_document(&mut self, cx: &mut gpui::Context<'_, Self>) -> Result<String, String> {
        self.state.processing_state = DocumentProcessingState::Processing;
        self.state.processing_start_time = Some(std::time::Instant::now());
        cx.notify();

        let result = match &self.state.input_method {
            DocumentInputMethod::TextInput => {
                if self.state.content.trim().is_empty() {
                    Err("Content cannot be empty".to_string())
                } else {
                    Ok(self.state.content.clone())
                }
            }
            DocumentInputMethod::UrlImport => {
                if self.state.url.trim().is_empty() {
                    Err("URL cannot be empty".to_string())
                } else {
                    self.url_fetcher.fetch_url(&self.state.url).await
                }
            }
            DocumentInputMethod::FileUpload => {
                if let Some(file_content) = &self.state.file_content {
                    // Try to find appropriate processor based on file extension
                    let extension = self.state.selected_file
                        .as_ref()
                        .and_then(|file| file.split('.').last())
                        .unwrap_or("txt");

                    let mut content = file_content.clone();

                    for processor in &self.processors {
                        if processor.supported_extensions().contains(&extension.to_string()) {
                            match processor.process_document(&content, Some(self.state.custom_metadata.clone())).await {
                                Ok(processed) => {
                                    content = processed;
                                    break;
                                }
                                Err(e) => {
                                    log::warn!("Processor {} failed: {}", processor.name(), e);
                                    // Continue with other processors
                                }
                            }
                        }
                    }

                    Ok(content)
                } else {
                    Err("No file selected".to_string())
                }
            }
            DocumentInputMethod::Clipboard => {
                // Simulate clipboard content retrieval
                Ok("Clipboard content would appear here".to_string())
            }
        };

        match &result {
            Ok(_) => {
                self.state.processing_state = DocumentProcessingState::Completed;
            }
            Err(e) => {
                self.state.processing_state = DocumentProcessingState::Error(e.clone());
                self.event_emitter.update(cx, |emitter, cx| {
                    emitter.emit(AddDocumentModalEvent::ProcessingFailed {
                        error: e.clone(),
                        input_method: self.state.input_method.clone(),
                    }, cx);
                });
            }
        }

        self.state.last_update = std::time::Instant::now();
        cx.notify();

        result
    }

    /// Add document to context
    pub async fn add_to_context(&mut self, cx: &mut gpui::Context<'_, Self>) -> Result<Arc<ContextItem>, String> {
        // Validate form
        self.validate_form()?;

        // Process document
        let processed_content = self.process_document(cx).await?;

        // Create document
        let document = Arc::new(Document {
            id: Some(ulid::Ulid::new().to_string()),
            title: self.state.title.clone(),
            description: if self.state.summary.trim().is_empty() {
                None
            } else {
                Some(self.state.summary.trim().to_string())
            },
            body: processed_content,
            url: match &self.state.input_method {
                DocumentInputMethod::UrlImport => self.state.url.clone(),
                DocumentInputMethod::FileUpload => {
                    self.state.selected_file.clone().unwrap_or_default()
                }
                _ => format!("local://{}", ulid::Ulid::new()),
            },
            tags: if self.state.tags.is_empty() {
                None
            } else {
                Some(self.state.tags.clone())
            },
            rank: None,
            metadata: self.state.custom_metadata.clone(),
        });

        // Create context item
        let context_item = Arc::new(ContextItem {
            id: ulid::Ulid::new().to_string(),
            title: document.title.clone(),
            summary: document.description.clone(),
            content: document.body.clone(),
            context_type: ContextType::Document,
            created_at: Utc::now(),
            relevance_score: None,
            metadata: {
                let mut metadata = ahash::AHashMap::new();
                metadata.insert("source".to_string(), format!("{:?}", self.state.input_method));
                metadata.insert("url".to_string(), document.url.clone());
                if !document.tags.as_ref().unwrap_or(&vec![]).is_empty() {
                    metadata.insert("tags".to_string(), document.tags.as_ref().unwrap().join(", "));
                }
                metadata.extend(document.metadata.clone());
                metadata
            },
        });

        // Emit success event
        self.event_emitter.update(cx, |emitter, cx| {
            emitter.emit(AddDocumentModalEvent::DocumentAdded {
                document: document.clone(),
                context_item: context_item.clone(),
            }, cx);
        });

        // Close modal
        self.close(cx);

        Ok(context_item)
    }

    /// Simulate file selection
    pub fn select_file(&mut self, file_path: String) -> Result<(), String> {
        let extension = file_path
            .split('.')
            .last()
            .unwrap_or("txt")
            .to_lowercase();

        if !self.config.supported_extensions.contains(&extension) {
            return Err(format!("Unsupported file extension: {}", extension));
        }

        self.state.selected_file = Some(file_path.clone());

        // Simulate file content reading
        let content = match extension.as_str() {
            "txt" => "This is sample text file content.".to_string(),
            "md" => "# Sample Markdown\n\nThis is **markdown** content.".to_string(),
            _ => "Sample file content".to_string(),
        };

        self.state.file_content = Some(content);

        // Auto-populate title if empty
        if self.state.title.is_empty() {
            let filename = file_path
                .split('/')
                .last()
                .unwrap_or(&file_path);
            self.state.title = filename.split('.').next().unwrap_or(filename).to_string();
        }

        self.state.last_update = std::time::Instant::now();
        Ok(())
    }

    /// Validate form inputs
    fn validate_form(&mut self) -> Result<(), String> {
        let mut has_errors = false;

        // Validate title
        if self.state.title.trim().is_empty() {
            self.state.title_error = Some("Title is required".to_string());
            has_errors = true;
        } else {
            self.state.title_error = None;
        }

        // Validate content based on input method
        match &self.state.input_method {
            DocumentInputMethod::TextInput => {
                if self.state.content.trim().is_empty() {
                    self.state.content_error = Some("Content is required".to_string());
                    has_errors = true;
                } else {
                    self.state.content_error = None;
                }
            }
            DocumentInputMethod::UrlImport => {
                if self.state.url.trim().is_empty() {
                    self.state.url_error = Some("URL is required".to_string());
                    has_errors = true;
                } else if !self.state.url.starts_with("http://") && !self.state.url.starts_with("https://") {
                    self.state.url_error = Some("Invalid URL format".to_string());
                    has_errors = true;
                } else {
                    self.state.url_error = None;
                }
            }
            DocumentInputMethod::FileUpload => {
                if self.state.selected_file.is_none() {
                    self.state.content_error = Some("Please select a file".to_string());
                    has_errors = true;
                } else {
                    self.state.content_error = None;
                }
            }
            DocumentInputMethod::Clipboard => {
                // Clipboard content would be validated asynchronously
            }
        }

        if has_errors {
            Err("Please fix the validation errors".to_string())
        } else {
            Ok(())
        }
    }

    /// Reset form to initial state
    fn reset_form(&mut self) {
        self.state.processing_state = DocumentProcessingState::Idle;
        self.state.title.clear();
        self.state.content.clear();
        self.state.url.clear();
        self.state.summary.clear();
        self.state.tags.clear();
        self.state.custom_metadata.clear();
        self.state.selected_file = None;
        self.state.file_content = None;
        self.state.title_error = None;
        self.state.content_error = None;
        self.state.url_error = None;
        self.state.processing_start_time = None;
    }

    /// Check if modal is open
    pub fn is_open(&self) -> bool {
        self.state.is_open
    }

    /// Get current input method
    pub fn get_input_method(&self) -> DocumentInputMethod {
        self.state.input_method.clone()
    }

    /// Subscribe to events
    pub fn subscribe<F, C>(&self, cx: &mut C, callback: F) -> gpui::Subscription
    where
        C: AppContext,
        F: Fn(&AddDocumentModalEvent, &mut C) + 'static,
    {
        cx.subscribe(&self.event_emitter, move |_, event, cx| {
            callback(event, cx);
        })
    }
}

