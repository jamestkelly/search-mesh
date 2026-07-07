# Roadmap

## Phase 0: Repository Foundation

- Replace template documentation.
- Add Rust workspace.
- Add formatting, linting, and test commands.
- Keep CI optional until the first implementation exists.

## Phase 1: Scan MVP

- Implement `scan`.
- Use `ignore` for repository traversal.
- Use `aho-corasick` for multi-keyword matching.
- Return file path, one-based line number, keyword, and matching line text.
- Add tests for overlapping keywords, missing directories, and invalid requests.

## Phase 2: MCP Server

- Implement JSON-RPC stdio loop.
- Support MCP `tools/list` and `tools/call`.
- Dispatch `scan` through `search-mesh-core`.

## Phase 3: AST Probe

- Add tree-sitter runtime. Done for Rust, Python, JavaScript, and TypeScript.
- Validate matches against node types. Done for initial aliases.
- Expand supported languages based on agent use cases.

## Phase 4: Squeezer

- Traverse parent AST nodes from a match coordinate. Done for the initial supported languages.
- Return the smallest self-contained structural block. Done for exact AST node text.
- Preserve enough imports or enclosing context only when required.

## Phase 5: Patch

- Translate line/column edits to byte offsets. Done.
- Apply in-memory edits. Done.
- Reparse supported syntax and report validity. Done.

## Phase 6: Performance Work

- Add benchmarks.
- Measure cold start, scan throughput, memory usage, and large-repo behavior.
- Consider mmap, Rayon, SIMD prefilters, and platform-specific acceleration only where benchmarks justify them.
