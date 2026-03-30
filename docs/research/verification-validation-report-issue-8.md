# Verification and Validation Report: Issue #8

**Issue**: Add OpenAPI documentation to axum server using utoipa
**Repository**: terraphim/terraphim-ai
**Date**: 2026-03-11
**Status**: NOT IMPLEMENTED

---

## Executive Summary

Issue #8 (opened 2024-01-07) requests OpenAPI documentation for the axum server using the `utoipa` crate. The server currently has no OpenAPI documentation. While there is a prototype using `poem_openapi` in the lab directory, the main terraphim_server uses axum without any OpenAPI integration.

| Requirement | Status | Evidence |
|-------------|--------|----------|
| utoipa dependency added | NOT MET | Not in Cargo.toml |
| OpenAPI spec generation | NOT MET | No utoipa attributes |
| API documentation | PARTIAL | Inline docs exist, but no OpenAPI |
| Swagger UI endpoint | NOT MET | Not implemented |

---

## Detailed Verification

### 1. utoipa Dependency Present

**Status**: NOT MET

**Evidence**:
```bash
$ grep -r "utoipa" crates/*/Cargo.toml terraphim_server/Cargo.toml
No matches found
```

**Current Dependencies in terraphim_server/Cargo.toml**:
- `axum = { version = "0.8.7", features = ["macros", "ws"] }` - web framework
- `schemars = { version = "0.8.22", optional = true }` - JSON schema generation (feature = "schema")
- No `utoipa` dependency

---

### 2. OpenAPI Spec Generation

**Status**: NOT MET

**Evidence**:
File: `terraphim_server/src/api.rs` (lines 1-100)

The API handlers use standard axum patterns without utoipa attributes:
```rust
pub(crate) async fn create_document(
    State(app_state): State<AppState>,
    Json(document): Json<Document>,
) -> Result<Json<CreateDocumentResponse>> {
    // ...
}

pub(crate) async fn search_documents(
    Extension(_tx): Extension<SearchResultsStream>,
    State(app_state): State<AppState>,
    search_query: Query<SearchQuery>,
) -> Result<Json<SearchResponse>> {
    // ...
}
```

**Missing utoipa annotations**:
- No `#[utoipa::path()]` attributes on handlers
- No `#[derive(OpenApi)]` on API struct
- No `#[derive(ToSchema)]` on request/response types

---

### 3. API Documentation

**Status**: PARTIAL

**Evidence**:

Inline documentation exists:
```rust
/// Health check endpoint
pub(crate) async fn health() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

/// Creates index of the document for each rolegraph
pub(crate) async fn create_document(...) -> Result<Json<CreateDocumentResponse>> {
    // ...
}

/// Search for documents in all Terraphim graphs defined in the config via GET params
pub(crate) async fn search_documents(...) -> Result<Json<SearchResponse>> {
    // ...
}
```

But no OpenAPI-specific documentation or schema generation.

---

### 4. Swagger UI Endpoint

**Status**: NOT MET

**Evidence**:
File: `terraphim_server/src/lib.rs` (router setup)

No Swagger UI endpoint exists. The server serves embedded frontend assets but no API documentation UI.

---

## Prototype in Lab Directory

**File**: `lab/parking-lot/server-poem/src/api.rs`

A prototype exists using `poem_openapi` (different crate than utoipa):
```rust
use poem_openapi::{payload::Json, ApiResponse, OpenApi};

#[derive(ApiResponse, Debug, PartialEq, Eq)]
pub enum QueryResponse {
    #[oai(status = 200)]
    Ok(PlainText<String>),
    #[oai(status = 404)]
    NotFound,
}

#[OpenApi]
impl Api {
    #[oai(path = "/documents", method = "post", tag = "ApiTags::Document")]
    async fn create_document(&self, document: Json<Document>) -> CreateDocumentResponse {
        // ...
    }
}
```

This is a **different approach** using the poem framework, not utoipa with axum.

---

## Current API Routes

Based on `terraphim_server/src/lib.rs`, the following routes exist:

| Method | Path | Handler |
|--------|------|---------|
| GET | /health | health |
| POST | /documents | create_document |
| POST | /documents/search | search_documents_post |
| GET | /documents/search | search_documents |
| GET | /documents/:id | get_document |
| PUT | /documents/:id | update_document |
| DELETE | /documents/:id | delete_document |
| GET | /config | get_config |
| POST | /config | post_config |
| GET | /roles | list_roles |
| GET | /kg/json/:role | get_rolegraph_json |
| POST | /chat | chat |
| POST | /summarize | summarize |
| GET | /ws | websocket handler |

---

## Alternative: schemars Feature

The codebase has an optional `schema` feature using `schemars`:

**Cargo.toml line 78**:
```toml
schema = ["dep:schemars", "terraphim_config/typescript"]
```

This generates JSON Schema but NOT OpenAPI. Different standard.

---

## Traceability Matrix

| Requirement | Design Element | Code Location | Test Coverage | Status |
|-------------|----------------|---------------|---------------|--------|
| utoipa dependency | MISSING | Cargo.toml | N/A | NOT MET |
| OpenApi derive | MISSING | lib.rs | N/A | NOT MET |
| path annotations | MISSING | api.rs | N/A | NOT MET |
| ToSchema derives | MISSING | types | N/A | NOT MET |
| Swagger UI | MISSING | lib.rs | N/A | NOT MET |
| Inline docs | Rust docs | api.rs | N/A | PARTIAL |

---

## Defect Register

| ID | Description | Severity | Resolution | Status |
|----|-------------|----------|------------|--------|
| D001 | No utoipa dependency | Medium | Add to Cargo.toml | OPEN |
| D002 | No utoipa path annotations | Medium | Add to handlers | OPEN |
| D003 | No ToSchema derives | Medium | Derive for types | OPEN |
| D004 | No OpenApi struct | Medium | Create API spec struct | OPEN |
| D005 | No Swagger UI endpoint | Low | Add /swagger-ui route | OPEN |

---

## Recommendations

### Option 1: Implement utoipa Integration

**Effort**: 1-2 days

**Steps**:
1. Add `utoipa` and `utoipa-swagger-ui` dependencies to terraphim_server/Cargo.toml
2. Add `#[derive(ToSchema)]` to request/response types:
   - `Document`
   - `SearchQuery`
   - `CreateDocumentResponse`
   - `SearchResponse`
   - etc.
3. Add `#[utoipa::path()]` attributes to handlers
4. Create OpenApi spec struct with `#[derive(OpenApi)]`
5. Add Swagger UI route

**Example of changes needed**:
```rust
// Before
pub(crate) async fn health() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}

// After
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Server is healthy")
    )
)]
pub(crate) async fn health() -> impl IntoResponse {
    (StatusCode::OK, "OK")
}
```

### Option 2: Use Existing schemars Feature

If JSON Schema is sufficient (not OpenAPI specifically):
- Document the existing `schema` feature
- Add schema generation endpoint
- Use tools that accept JSON Schema

### Option 3: Close as Not Planned

If API documentation is not a current priority:
- Close issue with explanation
- Document the inline Rust docs as the canonical API reference
- Note that the poem prototype in lab/ shows an alternative approach

---

## Conclusion

Issue #8 represents **unimplemented feature work**. The terraphim_server uses axum with inline documentation but has no OpenAPI integration. A prototype exists using poem_openapi in the lab directory, but this is a different framework than the production axum server.

### GO/NO-GO Decision: NO-GO

**Reasoning**:
- Feature request is not implemented
- No utoipa integration exists
- No OpenAPI spec generation
- Prototype uses different framework (poem vs axum)

**Next Steps**:
1. If implementing: Follow Option 1 steps above
2. If not implementing: Close with explanation about existing schemars feature
3. If deferring: Add to backlog with priority label

---

## Appendix: Files Referenced

| File | Path | Purpose |
|------|------|---------|
| Server Cargo.toml | `terraphim_server/Cargo.toml` | Dependencies (no utoipa) |
| API handlers | `terraphim_server/src/api.rs` | Axum handlers (no OpenAPI attrs) |
| Server lib | `terraphim_server/src/lib.rs` | Router setup |
| Poem prototype | `lab/parking-lot/server-poem/src/api.rs` | Alternative OpenAPI approach |
