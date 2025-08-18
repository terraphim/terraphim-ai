# RIPGREP: Haystack Integration Guide

RIPGREP: This document explains how haystack integration works in the Terraphim system. Haystacks are the backend storage systems that Terraphim uses to index and search documents.

## Haystack Types

### Ripgrep Haystack
- **Type**: File system based
- **Service**: Ripgrep
- **Location**: Local filesystem paths
- **Capabilities**: Full-text search using ripgrep
- **Prefix**: Documents from this haystack are prefixed with "RIPGREP:"

### Atomic Server Haystack  
- **Type**: Atomic Server based
- **Service**: Atomic
- **Location**: HTTP URLs
- **Capabilities**: Structured data search with atomic server
- **Prefix**: Documents from this haystack are prefixed with "ATOMIC:"

## Configuration

Haystacks are configured per role in the Terraphim configuration system. Each role can have multiple haystacks for comprehensive search coverage.

This RIPGREP document demonstrates the filesystem-based haystack functionality. 