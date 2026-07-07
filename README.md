# Search-Mesh

Search-Mesh is a Rust-native codebase intelligence service for autonomous coding agents.

The goal is to reduce agent latency and token waste by moving repository search, syntax-aware filtering, context extraction, and precise patching into a local MCP-compatible process.

## Status

This repository is in early setup. The current focus is a small, correct MVP:

- A Rust workspace with separate core and MCP crates.
- A JSON-RPC over stdio server suitable for Claude Code, OpenCode, and other MCP-compatible agents.
- A first search tool, `scan`, backed by multi-keyword scanning.

SIMD acceleration, tree-sitter verification, semantic squeezing, and atomic patching are planned phases, not current guarantees.

## Intended Tools

- `scan`: scan target directories for multiple keywords in one pass.
- `ast_probe`: validate raw hits against syntax tree node types.
- `squeeze`: return the smallest useful AST-bounded code block around a hit.
- `patch`: apply byte-offset edits and verify syntax after mutation.

See `docs/usage.md` for local examples and `docs/mcp-protocol.md` for the draft protocol.

## Repository Layout

```text
crates/
  search-mesh-core/   Core search, parsing, squeezing, and patching logic.
  search-mesh-mcp/    JSON-RPC/MCP stdio server.
docs/
  architecture.md     System shape and phased design.
  mcp-protocol.md     Tool schemas and response shapes.
  roadmap.md          Near-term implementation plan.
  usage.md            Local MCP usage examples.
examples/
  scan-request.jsonl  Example newline-delimited JSON-RPC requests.
```

## Development

Install Rust with `rustup`, then run:

```sh
just check
```

Equivalent commands:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Design Principles

- Prefer the smallest correct implementation before optimization.
- Keep protocol handling separate from core repository intelligence.
- Measure performance before adding specialized acceleration paths.
- Avoid `.unwrap()` and `.expect()` outside tests.
- Return agent-friendly payloads instead of whole files when possible.

## License

MIT. See `LICENSE`.
