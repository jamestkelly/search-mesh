# Usage

Search-Mesh currently exposes a newline-delimited JSON-RPC stdio server through the `search-mesh-mcp` binary.

## Run The Server With An Example Request

From the repository root:

```sh
cargo run --package search-mesh-mcp < examples/scan-request.jsonl
```

The example sends two requests:

- `tools/list`: lists available tools.
- `tools/call`: calls `scan` across `crates/` and `docs/`.

Each input line is one JSON-RPC request. Each output line is one JSON-RPC response.

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
