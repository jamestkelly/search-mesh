# Architecture

Search-Mesh is designed as a local Rust service that exposes codebase intelligence tools to coding agents over JSON-RPC stdio.

## Goal

Agents should not need to spawn many short-lived shell processes or ingest entire source files to answer targeted code questions. Search-Mesh should keep high-volume repository work local and return compact, actionable payloads.

## Crate Boundaries

```text
search-mesh-core
  Owns repository scanning, syntax probing, context extraction, and patch logic.

search-mesh-mcp
  Owns JSON-RPC stdio transport, MCP tool dispatch, request validation, and response serialization.
```

The core crate must not depend on the MCP crate. This keeps the engine testable without protocol plumbing.

## Pipeline

1. Scan raw files for multiple keywords.
2. Probe matches against syntax tree structure.
3. Squeeze matches to logical enclosing code blocks.
4. Patch by byte offsets and verify the modified syntax tree.

## MVP Scope

The first useful version should implement only:

- JSON-RPC stdio server startup.
- `scan` request parsing.
- Recursive directory scanning with ignore-file support.
- Multi-keyword matching using the `aho-corasick` crate.
- Unit tests for match coordinates and response shape.

## Deferred Optimizations

The design specification describes SIMD prefilters, Hyperscan-style throughput, memory mapping, lock-free queues, tree-sitter verification, and syntax-aware patch validation. These are future phases and should be added only after the MVP is measurable.

## Performance Posture

Initial success is not peak throughput. Initial success is fewer agent round trips, stable protocol behavior, and correct compact results. Once that exists, benchmarks can guide optimization work.
