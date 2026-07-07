# MCP Protocol Draft

Search-Mesh communicates over JSON-RPC stdio using newline-delimited MCP-style requests.

## Method: `initialize`

Performs the MCP lifecycle handshake. Real MCP clients (Claude Code, OpenCode) send this before calling any tools.

The server echoes back whatever `protocolVersion` the client requests. If none is provided, it falls back to its own supported version.

### Request

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "initialize",
  "params": {
    "protocolVersion": "2025-06-18",
    "capabilities": {},
    "clientInfo": { "name": "example-client", "version": "1.0.0" }
  }
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "protocolVersion": "2025-06-18",
    "capabilities": { "tools": {} },
    "serverInfo": { "name": "search-mesh", "version": "0.1.0" }
  }
}
```

### `notifications/initialized`

After `initialize`, clients send a JSON-RPC notification (no `id`) to signal readiness:

```json
{"jsonrpc":"2.0","method":"notifications/initialized"}
```

Per JSON-RPC 2.0, notifications never receive a response. Search-Mesh writes nothing to stdout for any message without an `id`.

## Method: `tools/list`

Lists available tools.

### Request

```json
{"jsonrpc":"2.0","id":1,"method":"tools/list"}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "tools": [
      {
        "name": "scan",
        "description": "Scan target directories for multiple keywords.",
        "inputSchema": {
          "type": "object",
          "properties": {
            "targetDirs": {
              "type": "array",
              "items": { "type": "string" }
            },
            "keywords": {
              "type": "array",
              "items": { "type": "string" }
            }
          },
          "required": ["targetDirs", "keywords"]
        }
      },
      {
        "name": "ast_probe",
        "description": "Validate whether a pattern appears inside a requested syntax node type.",
        "inputSchema": {
          "type": "object",
          "properties": {
            "filePath": { "type": "string" },
            "queryPattern": { "type": "string" },
            "nodeType": { "type": "string" }
          },
          "required": ["filePath", "queryPattern", "nodeType"]
        }
      },
      {
        "name": "squeeze",
        "description": "Return the AST-bounded source block around a matched pattern.",
        "inputSchema": {
          "type": "object",
          "properties": {
            "filePath": { "type": "string" },
            "queryPattern": { "type": "string" },
            "nodeType": { "type": "string" }
          },
          "required": ["filePath", "queryPattern", "nodeType"]
        }
      },
      {
        "name": "patch",
        "description": "Apply a 1-based line/column text replacement and report syntax validity.",
        "inputSchema": {
          "type": "object",
          "properties": {
            "filePath": { "type": "string" },
            "startLine": { "type": "integer" },
            "startColumn": { "type": "integer" },
            "endLine": { "type": "integer" },
            "endColumn": { "type": "integer" },
            "replacement": { "type": "string" }
          },
          "required": [
            "filePath",
            "startLine",
            "startColumn",
            "endLine",
            "endColumn",
            "replacement"
          ]
        }
      }
    ]
  }
}
```

## Tool: `scan`

Scans target directories for multiple keywords and returns line-oriented matches.

### Request

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/call",
  "params": {
    "name": "scan",
    "arguments": {
      "targetDirs": ["src/"],
      "keywords": ["TODO", "FIXME", "deprecated"]
    }
  }
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "[{\"file\":\"src/main.rs\",\"line\":142,\"keyword\":\"TODO\",\"matchStr\":\"// TODO: Refactor AhoNode initialization\"}]"
      }
    ]
  }
}
```

## Tool: `ast_probe`

Validates whether a pattern appears in a requested syntax node type.

Supported file extensions: `.rs`, `.py`, `.js`, `.jsx`, `.mjs`, `.cjs`, `.ts`, `.tsx`.

Supported aliases:

- Rust: `function`, `struct`, `impl`, `enum`
- Python: `function`, `class`
- JavaScript: `function`, `class`
- TypeScript: `function`, `class`, `interface`

Raw tree-sitter node kinds may also be passed as `nodeType`.

### Request

```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "ast_probe",
    "arguments": {
      "filePath": "src/main.rs",
      "queryPattern": "AhoNode",
      "nodeType": "struct"
    }
  }
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\"isValid\":true,\"nodeType\":\"struct_item\",\"startLine\":12,\"endLine\":48}"
      }
    ]
  }
}
```

## Tool: `squeeze`

Returns the nearest useful AST-bounded block around a match.

Uses the same file extensions and `nodeType` aliases as `ast_probe`. If no matching block is found, the MCP text payload contains `null`.

### Request

```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "method": "tools/call",
  "params": {
    "name": "squeeze",
    "arguments": {
      "filePath": "src/main.rs",
      "queryPattern": "delete_user",
      "nodeType": "function"
    }
  }
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\"file\":\"src/main.rs\",\"nodeType\":\"function_item\",\"startLine\":31,\"endLine\":74,\"text\":\"pub fn delete_user() {\\n    // ...\\n}\"}"
      }
    ]
  }
}
```

## Tool: `patch`

Applies a 1-based line/column text replacement and reports syntax validity.

The edit range is end-exclusive. For supported syntax extensions (`.rs`, `.py`, `.js`, `.jsx`, `.mjs`, `.cjs`, `.ts`, `.tsx`), `syntaxValid` is `true` or `false`. For unsupported extensions, `syntaxValid` is `null`.

### Request

```json
{
  "jsonrpc": "2.0",
  "id": 5,
  "method": "tools/call",
  "params": {
    "name": "patch",
    "arguments": {
      "filePath": "src/main.rs",
      "startLine": 10,
      "startColumn": 5,
      "endLine": 10,
      "endColumn": 12,
      "replacement": "new_name"
    }
  }
}
```

### Response

```json
{
  "jsonrpc": "2.0",
  "id": 5,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\"file\":\"src/main.rs\",\"bytesWritten\":1234,\"syntaxValid\":true}"
      }
    ]
  }
}
```

## Error Shape

Tool failures should use JSON-RPC errors at the transport layer when request dispatch fails. Tool-level validation errors should return an MCP content payload with a concise message until the server has a fuller typed error model.
