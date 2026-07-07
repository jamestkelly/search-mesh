# Usage

Search-Mesh currently exposes a newline-delimited JSON-RPC stdio server through the `search-mesh-mcp` binary.

See the README's [Install](../README.md#install) section for `cargo install`/binary download instructions, the Claude Code plugin, and manual OpenCode/Claude Code configuration. See `skills/search-mesh/SKILL.md` for an agent skill template describing when to prefer these tools over shell commands.

## Run The Server With An Example Request

From the repository root:

```sh
cargo run --package search-mesh-mcp < examples/scan-request.jsonl
```

The example sends two requests:

- `tools/list`: lists available tools.
- `tools/call`: calls `scan` across `crates/` and `docs/`.

Each input line is one JSON-RPC request. Each output line is one JSON-RPC response.

## Run The Initialize Handshake Example

Real MCP clients send `initialize` before calling any tools. This example demonstrates the handshake, including a `notifications/initialized` notification that correctly produces no output:

```sh
cargo run --package search-mesh-mcp < examples/initialize-request.jsonl
```

You should see exactly two output lines (for the `initialize` and `tools/list` requests) — nothing for the notification line in between.

## Run The AST Probe Example

```sh
cargo run --package search-mesh-mcp < examples/ast-probe-request.jsonl
```

## Run The Squeeze Example

```sh
cargo run --package search-mesh-mcp < examples/squeeze-request.jsonl
```

## Run The Patch Example

The patch example writes to `/tmp/search-mesh-patch-target.txt` so the repository fixture remains unchanged:

```sh
cp examples/patch-target.txt /tmp/search-mesh-patch-target.txt
cargo run --package search-mesh-mcp < examples/patch-request.jsonl
```

## List Tools

```sh
printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' \
  | cargo run --quiet --package search-mesh-mcp
```

## Call `scan`

```sh
printf '%s\n' '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"scan","arguments":{"targetDirs":["crates/"],"keywords":["TODO","FIXME","scan"]}}}' \
  | cargo run --quiet --package search-mesh-mcp
```

The `scan` result is returned as an MCP content payload. The `text` field contains a JSON array of matches:

```json
[
  {
    "file": "crates/search-mesh-core/src/scan.rs",
    "line": 31,
    "keyword": "scan",
    "matchStr": "pub fn scan_keywords(request: &ScanRequest) -> Result<Vec<ScanMatch>, ScanError> {"
  }
]
```

## Tool Arguments

`scan` accepts:

- `targetDirs`: array of directories to walk.
- `keywords`: array of case-sensitive keywords to find.

The scanner respects ignore files, skips non-files, and skips unreadable or non-UTF-8 files.

## Call `ast_probe`

```sh
printf '%s\n' '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"ast_probe","arguments":{"filePath":"crates/search-mesh-core/src/scan.rs","queryPattern":"scan_keywords","nodeType":"function"}}}' \
  | cargo run --quiet --package search-mesh-mcp
```

The `ast_probe` result is returned as an MCP content payload. The `text` field contains a JSON object:

```json
{
  "isValid": true,
  "nodeType": "function_item",
  "startLine": 31,
  "endLine": 74
}
```

Supported file extensions:

- Rust: `.rs`
- Python: `.py`
- JavaScript: `.js`, `.jsx`, `.mjs`, `.cjs`
- TypeScript: `.ts`, `.tsx`

Supported aliases:

- Rust: `function`, `struct`, `impl`, `enum`
- Python: `function`, `class`
- JavaScript: `function`, `class`
- TypeScript: `function`, `class`, `interface`

You can also pass a raw tree-sitter node kind as `nodeType` when no alias exists.

## Call `squeeze`

```sh
printf '%s\n' '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"squeeze","arguments":{"filePath":"crates/search-mesh-core/src/scan.rs","queryPattern":"scan_keywords","nodeType":"function"}}}' \
  | cargo run --quiet --package search-mesh-mcp
```

The `squeeze` result is returned as an MCP content payload. The `text` field contains a JSON object with the AST-bounded source block:

```json
{
  "file": "crates/search-mesh-core/src/scan.rs",
  "nodeType": "function_item",
  "startLine": 31,
  "endLine": 74,
  "text": "pub fn scan_keywords(...) { ... }"
}
```

If no matching AST block is found, the `text` field contains `null`.

## Call `patch`

```sh
cp examples/patch-target.txt /tmp/search-mesh-patch-target.txt
printf '%s\n' '{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"patch","arguments":{"filePath":"/tmp/search-mesh-patch-target.txt","startLine":1,"startColumn":7,"endLine":1,"endColumn":10,"replacement":"new"}}}' \
  | cargo run --quiet --package search-mesh-mcp
```

The `patch` result is returned as an MCP content payload. The `text` field contains a JSON object:

```json
{
  "file": "/tmp/search-mesh-patch-target.txt",
  "bytesWritten": 10,
  "syntaxValid": null
}
```

Patch coordinates are 1-based. The end coordinate is exclusive.
