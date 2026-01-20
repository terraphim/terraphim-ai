+++
title="Building a GitHub Actions-Style Runner with Firecracker VMs and Knowledge Graph Learning"
date=2025-12-25

[taxonomies]
categories = ["Engineering", "Rust", "CI"]
tags = ["Terraphim", "firecracker", "github-actions", "microvm", "knowledge-graph"]
[extra]
toc = true
comments = true
+++

We built a production-ready GitHub Actions-style workflow runner that combines Firecracker microVM isolation with knowledge-graph-driven learning about what actually works.

<!-- more -->

## Overview

The `terraphim_github_runner` crate provides a complete system for:

1. Processing GitHub webhook events into executable workflows
2. Spawning and managing Firecracker microVMs for isolated command execution
3. Tracking command execution patterns in a knowledge graph
4. Learning from success/failure to improve future workflows

## Architecture

### High-Level Data Flow

```
GitHub Webhook -> WorkflowContext -> ParsedWorkflow -> SessionManager
                                              |
                                          Create VM
                                              |
                              Execute Commands (VmCommandExecutor)
                                              |
                            +-----------------+-----------------+
                            |                                   |
                    LearningCoordinator                  CommandKnowledgeGraph
                    (success/failure stats)              (pattern learning)
```

## Key Components

### 1. VM Executor

The VM executor is the bridge to Firecracker.

### 2. Command Knowledge Graph

The knowledge graph tracks command execution patterns.

## Source Material

- Source: `docs/archive/blog-posts/github-runner-architecture.md`
