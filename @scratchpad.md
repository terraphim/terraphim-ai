# Scratchpad - KG Term to Document Lookup Implementation

## Current Task: Create API and Tauri command to find source documents from KG terms

**Example Use Case**: Given "haystack" term → find `haystack.md` document (which has synonyms: datasource, service, agent)

## Implementation Plan Status:
- [x] Move Document struct to terraphim_types with TypeScript bindings ✅ ALREADY DONE
- [x] Add reverse document ID mapping in RoleGraph ✅ COMPLETED
- [x] Create KG term query method in RoleGraph ✅ COMPLETED
- [x] Enhance persistence layer with batch document lookup ✅ COMPLETED
- [x] Create service method for KG term to document lookup ✅ COMPLETED
- [x] Add API endpoint: GET /roles/{role_name}/kg_search ✅ COMPLETED
- [x] Create Tauri command for desktop app ✅ COMPLETED
- [x] Add comprehensive integration test ✅ COMPLETED
- [x] Generate TypeScript bindings ✅ COMPLETED

## Key Insights:
- RoleGraph already has the mapping from terms to source documents
- Need to expose reverse lookup capability: term → document IDs → Document objects
- haystack.md example shows synonyms are stored and should be searchable
- Must maintain type safety between Rust backend and TypeScript frontend
- Document struct already exists in terraphim_types with TypeScript bindings ✅
- RoleGraph.insert_document() creates node-document relationships via edges
- Edge.doc_hash contains document_id -> rank mapping
- Need to traverse: term → node_id → edges → document_ids

## ✅ IMPLEMENTATION COMPLETED SUCCESSFULLY!

### Summary of what was implemented:

**Core Functionality:**
1. **RoleGraph Enhancement**: Added `find_document_ids_for_term()` method to find source documents for any KG term
2. **Persistence Layer**: Added `load_documents_by_ids()` function for efficient batch document loading
3. **Service Layer**: Created `find_documents_for_kg_term()` method in TerraphimService
4. **API Endpoint**: Added `GET /roles/{role_name}/kg_search?term=<term>` endpoint in terraphim_server
5. **Tauri Command**: Created `find_documents_for_kg_term` command for desktop app integration
6. **TypeScript Bindings**: Generated `DocumentListResponse` type for frontend

**Testing:**
- Created comprehensive integration test (`kg_term_to_document_test.rs`)
- Tests validate complete flow from API → service → rolegraph → persistence
- Tests include edge cases (invalid roles, non-existent terms)

**Example Usage:**
- API: `GET /roles/Terraphim%20Engineer/kg_search?term=haystack`
- Tauri: `invoke('find_documents_for_kg_term', { role_name: 'Terraphim Engineer', term: 'haystack' })`
- Returns: Documents that contain "haystack" or its synonyms ("datasource", "service", "agent")

**Key Achievement:** Complete bidirectional linking between KG terms and source documents!
🎉 From "haystack" term → finds "haystack.md" document with full content and metadata.