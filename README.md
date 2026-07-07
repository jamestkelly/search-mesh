# Search-Mesh

Search-Mesh is a Rust-native codebase intelligence service for autonomous coding agents.

The goal is to reduce agent latency and token waste by moving repository search, syntax-aware filtering, context extraction, and precise patching into a local MCP-compatible process.

## Status

This repository is in early setup. The current focus is a small, correct MVP:

- A Rust workspace with separate core and MCP crates.
- A JSON-RPC over stdio server implementing the MCP `initialize` lifecycle handshake, suitable for Claude Code, OpenCode, and other MCP-compatible agents.
- A first search tool, `scan`, backed by multi-keyword scanning.
- A syntax probe tool, `ast_probe`, backed by tree-sitter for Rust, Python, JavaScript, and TypeScript.
- A context extraction tool, `squeeze`, that returns AST-bounded source blocks.
- A patch tool, `patch`, that applies precise line/column edits and reports syntax validity.

SIMD acceleration, tree-sitter verification, semantic squeezing, and atomic patching are planned phases, not current guarantees.

## Intended Tools

- `scan`: scan target directories for multiple keywords in one pass.
- `ast_probe`: validate raw hits against syntax tree node types.
- `squeeze`: return the smallest useful AST-bounded code block around a hit.
- `patch`: apply byte-offset edits and verify syntax after mutation.

See `docs/usage.md` for local examples and `docs/mcp-protocol.md` for the draft protocol.

## Install

### Claude Code Plugin (Recommended)

This repository is a self-hosted Claude Code plugin marketplace. It bundles the `search-mesh` MCP server registration and the agent skill together, so `/plugin install` wires up both in one step:

```
/plugin marketplace add jamestkelly/search-mesh
/plugin install search-mesh@search-mesh
```

On macOS and Linux, the plugin downloads and verifies the correct prebuilt `search-mesh-mcp` binary automatically the first time it's used — no separate install step. Windows isn't supported by the automatic installer yet; use `cargo install search-mesh-mcp` or a manual binary install there.

### Manual Install

For OpenCode, or Claude Code configured manually rather than through the plugin, `search-mesh-mcp` needs to be on your `PATH` first.

`search-mesh-mcp` is published to crates.io and as prebuilt GitHub Release binaries.

**Via cargo:**

```sh
cargo install search-mesh-mcp
```

**Via prebuilt binary:** download `search-mesh-mcp-<target>.tar.gz` for your platform (macOS arm64, macOS x64, or Linux x64) from the [Releases page](https://github.com/jamestkelly/search-mesh/releases), extract it, and put `search-mesh-mcp` on your `PATH`.

#### Configure Claude Code Manually

Add to `.mcp.json` at your project root:

```json
{
  "mcpServers": {
    "search-mesh": {
      "command": "search-mesh-mcp",
      "args": []
    }
  }
}
```

#### Configure OpenCode

Add to `opencode.jsonc`:

```jsonc
{
  "mcp": {
    "search-mesh": {
      "type": "local",
      "command": ["search-mesh-mcp"],
      "enabled": true
    }
  }
}
```

### Agent Skill

The Claude Code plugin install above already includes this skill. To install it manually for other agents/setups, `skills/search-mesh/SKILL.md` teaches an agent when to prefer `scan`, `ast_probe`, `squeeze`, and `patch` over shell tools like `grep`, `cat`, and `sed`. Copy it into your own skills directory:

```sh
mkdir -p ~/.claude/skills/search-mesh
cp skills/search-mesh/SKILL.md ~/.claude/skills/search-mesh/SKILL.md
```

## Repository Layout

```text
.claude-plugin/
  plugin.json       Claude Code plugin manifest.
  marketplace.json  Self-hosted marketplace catalog listing this plugin.
.mcp.json           Bundled MCP server registration (used by the plugin).
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
  initialize-request.jsonl
  ast-probe-request.jsonl
  squeeze-request.jsonl
  patch-request.jsonl
  patch-target.txt
scripts/
  search-mesh-mcp-launcher.sh  Installs search-mesh-mcp on first use (macOS/Linux), then execs it.
skills/
  search-mesh/SKILL.md  Agent skill teaching when to use these tools.
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

## CI And Releases

Pull requests and pushes to `main` run the `Review` workflow:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace`
- MCP JSONL example smoke tests

Releases are prepared with `release-plz` from Conventional Commits:

- `release-plz` opens a release PR that bumps `Cargo.toml`/`Cargo.lock` versions and updates the changelog.
- Merging that PR to `main` creates a git tag and GitHub Release per changed package, and publishes both crates to crates.io.
- When a `search-mesh-mcp-v*` release is published, the `Release Binaries` workflow builds and attaches `search-mesh-mcp` binaries (plus a `.sha256` checksum file for each) for macOS (arm64, x64) and Linux (x64) as release assets.

## Design Principles

- Prefer the smallest correct implementation before optimization.
- Keep protocol handling separate from core repository intelligence.
- Measure performance before adding specialized acceleration paths.
- Avoid `.unwrap()` and `.expect()` outside tests.
- Return agent-friendly payloads instead of whole files when possible.

## License

MIT. See `LICENSE`.
