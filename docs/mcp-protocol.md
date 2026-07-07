# MCP Protocol Draft

Search-Mesh communicates over JSON-RPC stdio using newline-delimited MCP-style requests.

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

Status: planned.

## Tool: `patch`

Applies byte-offset edits and verifies the resulting file.

Status: planned.

## Error Shape

Tool failures should use JSON-RPC errors at the transport layer when request dispatch fails. Tool-level validation errors should return an MCP content payload with a concise message until the server has a fuller typed error model.
