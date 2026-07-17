use std::{io, path::PathBuf};

use search_mesh_core::{
    PatchRequest, PatchResponse, ProbeRequest, ProbeResponse, RenameRequest, RenameResponse,
    ScanMatch, ScanRequest, SqueezeRequest, SqueezeResponse, apply_patch, apply_rename, ast_probe,
    scan_keywords, squeeze,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;

const SUPPORTED_PROTOCOL_VERSION: &str = "2025-06-18";

#[derive(Debug, Error)]
pub enum ServerError {
    #[error(transparent)]
    Io(#[from] io::Error),
}

#[derive(Debug, Deserialize)]
struct JsonRpcRequest {
    id: Option<Value>,
    method: String,
    #[serde(default)]
    params: Value,
}

#[derive(Debug, Deserialize)]
struct ToolCallParams {
    name: String,
    #[serde(default)]
    arguments: Value,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScanArgs {
    target_dirs: Vec<PathBuf>,
    keywords: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AstProbeArgs {
    file_path: PathBuf,
    query_pattern: String,
    node_type: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SqueezeArgs {
    file_path: PathBuf,
    query_pattern: String,
    node_type: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PatchArgs {
    file_path: PathBuf,
    start_line: usize,
    start_column: usize,
    end_line: usize,
    end_column: usize,
    replacement: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanMatchPayload {
    file: String,
    line: usize,
    keyword: String,
    match_str: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProbeResponsePayload {
    is_valid: bool,
    node_type: Option<String>,
    start_line: Option<usize>,
    end_line: Option<usize>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SqueezeResponsePayload {
    file: String,
    node_type: String,
    start_line: usize,
    end_line: usize,
    text: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PatchResponsePayload {
    file: String,
    bytes_written: usize,
    syntax_valid: Option<bool>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RenameArgs {
    file_path: PathBuf,
    target: String,
    replacement: String,
    node_type: Option<String>,
    query_pattern: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RenameResponsePayload {
    file: String,
    bytes_written: usize,
    occurrences_renamed: usize,
    syntax_valid: Option<bool>,
}

pub fn handle_jsonrpc(input: &str) -> Option<String> {
    let request = match serde_json::from_str::<JsonRpcRequest>(input) {
        Ok(request) => request,
        Err(error) => {
            return Some(jsonrpc_error(None, -32700, format!("parse error: {error}")).to_string());
        }
    };

    // JSON-RPC notifications (no `id`) never receive a response.
    request.id.as_ref()?;

    Some(dispatch(request).to_string())
}

fn dispatch(request: JsonRpcRequest) -> Value {
    match request.method.as_str() {
        "initialize" => call_initialize(request.id, request.params),
        "tools/list" => jsonrpc_result(request.id, tools_list()),
        "tools/call" => dispatch_tool_call(request.id, request.params),
        method => jsonrpc_error(
            request.id,
            -32601,
            format!("unsupported JSON-RPC method: {method}"),
        ),
    }
}

fn call_initialize(id: Option<Value>, params: Value) -> Value {
    let protocol_version = params
        .get("protocolVersion")
        .and_then(Value::as_str)
        .unwrap_or(SUPPORTED_PROTOCOL_VERSION);

    jsonrpc_result(
        id,
        json!({
            "protocolVersion": protocol_version,
            "capabilities": {
                "tools": {}
            },
            "serverInfo": {
                "name": "search-mesh",
                "version": env!("CARGO_PKG_VERSION")
            }
        }),
    )
}

fn dispatch_tool_call(id: Option<Value>, params: Value) -> Value {
    let params = match serde_json::from_value::<ToolCallParams>(params) {
        Ok(params) => params,
        Err(error) => {
            return jsonrpc_error(id, -32602, format!("invalid tool call params: {error}"));
        }
    };

    match params.name.as_str() {
        "scan" => call_scan(id, params.arguments),
        "ast_probe" => call_ast_probe(id, params.arguments),
        "squeeze" => call_squeeze(id, params.arguments),
        "patch" => call_patch(id, params.arguments),
        "rename" => call_rename(id, params.arguments),
        name => jsonrpc_error(id, -32602, format!("unsupported tool: {name}")),
    }
}

fn call_scan(id: Option<Value>, arguments: Value) -> Value {
    let arguments = match serde_json::from_value::<ScanArgs>(arguments) {
        Ok(arguments) => arguments,
        Err(error) => {
            return jsonrpc_error(id, -32602, format!("invalid scan args: {error}"));
        }
    };

    let request = ScanRequest {
        target_dirs: arguments.target_dirs,
        keywords: arguments.keywords,
    };

    match scan_keywords(&request) {
        Ok(matches) => jsonrpc_result(id, content_payload(scan_matches_payload(matches))),
        Err(error) => jsonrpc_error(id, -32603, error.to_string()),
    }
}

fn call_ast_probe(id: Option<Value>, arguments: Value) -> Value {
    let arguments = match serde_json::from_value::<AstProbeArgs>(arguments) {
        Ok(arguments) => arguments,
        Err(error) => {
            return jsonrpc_error(id, -32602, format!("invalid ast_probe args: {error}"));
        }
    };

    let request = ProbeRequest {
        file_path: arguments.file_path,
        query_pattern: arguments.query_pattern,
        node_type: arguments.node_type,
    };

    match ast_probe(&request) {
        Ok(response) => jsonrpc_result(id, content_payload(probe_response_payload(response))),
        Err(error) => jsonrpc_error(id, -32603, error.to_string()),
    }
}

fn call_squeeze(id: Option<Value>, arguments: Value) -> Value {
    let arguments = match serde_json::from_value::<SqueezeArgs>(arguments) {
        Ok(arguments) => arguments,
        Err(error) => {
            return jsonrpc_error(id, -32602, format!("invalid squeeze args: {error}"));
        }
    };

    let request = SqueezeRequest {
        file_path: arguments.file_path,
        query_pattern: arguments.query_pattern,
        node_type: arguments.node_type,
    };

    match squeeze(&request) {
        Ok(Some(response)) => {
            jsonrpc_result(id, content_payload(squeeze_response_payload(response)))
        }
        Ok(None) => jsonrpc_result(id, content_payload(Value::Null)),
        Err(error) => jsonrpc_error(id, -32603, error.to_string()),
    }
}

fn call_patch(id: Option<Value>, arguments: Value) -> Value {
    let arguments = match serde_json::from_value::<PatchArgs>(arguments) {
        Ok(arguments) => arguments,
        Err(error) => {
            return jsonrpc_error(id, -32602, format!("invalid patch args: {error}"));
        }
    };

    let request = PatchRequest {
        file_path: arguments.file_path,
        start_line: arguments.start_line,
        start_column: arguments.start_column,
        end_line: arguments.end_line,
        end_column: arguments.end_column,
        replacement: arguments.replacement,
    };

    match apply_patch(&request) {
        Ok(response) => jsonrpc_result(id, content_payload(patch_response_payload(response))),
        Err(error) => jsonrpc_error(id, -32603, error.to_string()),
    }
}

fn call_rename(id: Option<Value>, arguments: Value) -> Value {
    let arguments = match serde_json::from_value::<RenameArgs>(arguments) {
        Ok(arguments) => arguments,
        Err(error) => {
            return jsonrpc_error(id, -32602, format!("invalid rename args: {error}"));
        }
    };

    let request = RenameRequest {
        file_path: arguments.file_path,
        target: arguments.target,
        replacement: arguments.replacement,
        node_type: arguments.node_type,
        query_pattern: arguments.query_pattern,
    };

    match apply_rename(&request) {
        Ok(response) => jsonrpc_result(id, content_payload(rename_response_payload(response))),
        Err(error) => jsonrpc_error(id, -32603, error.to_string()),
    }
}

fn rename_response_payload(response: RenameResponse) -> RenameResponsePayload {
    RenameResponsePayload {
        file: response.file.display().to_string(),
        bytes_written: response.bytes_written,
        occurrences_renamed: response.occurrences_renamed,
        syntax_valid: response.syntax_valid,
    }
}

fn scan_matches_payload(matches: Vec<ScanMatch>) -> Vec<ScanMatchPayload> {
    matches
        .into_iter()
        .map(|scan_match| ScanMatchPayload {
            file: scan_match.file.display().to_string(),
            line: scan_match.line,
            keyword: scan_match.keyword,
            match_str: scan_match.match_str.to_string(),
        })
        .collect()
}

fn probe_response_payload(response: ProbeResponse) -> ProbeResponsePayload {
    ProbeResponsePayload {
        is_valid: response.is_valid,
        node_type: response.node_type,
        start_line: response.start_line,
        end_line: response.end_line,
    }
}

fn squeeze_response_payload(response: SqueezeResponse) -> SqueezeResponsePayload {
    SqueezeResponsePayload {
        file: response.file.display().to_string(),
        node_type: response.node_type,
        start_line: response.start_line,
        end_line: response.end_line,
        text: response.text,
    }
}

fn patch_response_payload(response: PatchResponse) -> PatchResponsePayload {
    PatchResponsePayload {
        file: response.file.display().to_string(),
        bytes_written: response.bytes_written,
        syntax_valid: response.syntax_valid,
    }
}

fn tools_list() -> Value {
    json!({
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
            },
            {
                "name": "rename",
                "description": "Rename all occurrences of an exact identifier within a file, optionally scoped to a specific AST node.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "filePath": { "type": "string" },
                        "target": { "type": "string" },
                        "replacement": { "type": "string" },
                        "nodeType": { "type": "string" },
                        "queryPattern": { "type": "string" }
                    },
                    "required": ["filePath", "target", "replacement"]
                }
            }
        ]
    })
}

fn content_payload<T>(payload: T) -> Value
where
    T: Serialize,
{
    let text = match serde_json::to_string(&payload) {
        Ok(text) => text,
        Err(error) => format!("serialization error: {error}"),
    };

    json!({
        "content": [
            {
                "type": "text",
                "text": text
            }
        ]
    })
}

fn jsonrpc_result(id: Option<Value>, result: Value) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id.unwrap_or(Value::Null),
        "result": result
    })
}

fn jsonrpc_error(id: Option<Value>, code: i64, message: String) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id.unwrap_or(Value::Null),
        "error": {
            "code": code,
            "message": message
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    use tempfile::TempDir;

    fn response_value(input: &str) -> Result<Value, Box<dyn std::error::Error>> {
        let response =
            handle_jsonrpc(input).ok_or("expected a response for a request with an id")?;
        Ok(serde_json::from_str(&response)?)
    }

    #[test]
    fn handles_initialize_handshake() -> Result<(), Box<dyn std::error::Error>> {
        let response = response_value(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test-client","version":"1.0.0"}}}"#,
        )?;

        assert_eq!(response["id"], 1);
        assert_eq!(response["result"]["protocolVersion"], "2025-06-18");
        assert_eq!(response["result"]["serverInfo"]["name"], "search-mesh");
        assert!(response["result"]["capabilities"]["tools"].is_object());

        Ok(())
    }

    #[test]
    fn initialize_falls_back_to_default_protocol_version() -> Result<(), Box<dyn std::error::Error>>
    {
        let response =
            response_value(r#"{"jsonrpc":"2.0","id":2,"method":"initialize","params":{}}"#)?;

        assert_eq!(
            response["result"]["protocolVersion"],
            SUPPORTED_PROTOCOL_VERSION
        );

        Ok(())
    }

    #[test]
    fn returns_no_response_for_notifications() {
        let response = handle_jsonrpc(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#);

        assert_eq!(response, None);
    }

    #[test]
    fn lists_scan_tool() -> Result<(), Box<dyn std::error::Error>> {
        let response = response_value(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#)?;

        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 1);
        assert_eq!(response["result"]["tools"][0]["name"], "scan");
        assert_eq!(response["result"]["tools"][1]["name"], "ast_probe");
        assert_eq!(response["result"]["tools"][2]["name"], "squeeze");
        assert_eq!(response["result"]["tools"][3]["name"], "patch");

        Ok(())
    }

    #[test]
    fn calls_scan() -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        let source_path = root.path().join("src");
        fs::create_dir_all(&source_path)?;
        fs::write(source_path.join("main.rs"), "// TODO: wire MCP\n")?;

        let request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/call",
            "params": {
                "name": "scan",
                "arguments": {
                    "targetDirs": [source_path],
                    "keywords": ["TODO"]
                }
            }
        });
        let response = response_value(&request.to_string())?;
        let text = response["result"]["content"][0]["text"]
            .as_str()
            .ok_or("missing text content")?;
        let matches: Value = serde_json::from_str(text)?;

        assert_eq!(response["id"], 2);
        assert_eq!(matches[0]["line"], 1);
        assert_eq!(matches[0]["keyword"], "TODO");
        assert_eq!(matches[0]["matchStr"], "// TODO: wire MCP");

        Ok(())
    }

    #[test]
    fn rejects_unknown_tool() -> Result<(), Box<dyn std::error::Error>> {
        let response = response_value(
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"missing_tool"}}"#,
        )?;

        assert_eq!(response["error"]["code"], -32602);

        Ok(())
    }

    #[test]
    fn calls_ast_probe() -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        let file_path = root.path().join("lib.rs");
        fs::write(&file_path, "pub fn route_context() {\n}\n")?;

        let request = json!({
            "jsonrpc": "2.0",
            "id": 4,
            "method": "tools/call",
            "params": {
                "name": "ast_probe",
                "arguments": {
                    "filePath": file_path,
                    "queryPattern": "route_context",
                    "nodeType": "function"
                }
            }
        });
        let response = response_value(&request.to_string())?;
        let text = response["result"]["content"][0]["text"]
            .as_str()
            .ok_or("missing text content")?;
        let probe: Value = serde_json::from_str(text)?;

        assert_eq!(response["id"], 4);
        assert_eq!(probe["isValid"], true);
        assert_eq!(probe["nodeType"], "function_item");
        assert_eq!(probe["startLine"], 1);

        Ok(())
    }

    #[test]
    fn calls_squeeze() -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        let file_path = root.path().join("lib.rs");
        fs::write(
            &file_path,
            "fn other() {}\n\npub fn route_context() {\n    println!(\"ok\");\n}\n",
        )?;

        let request = json!({
            "jsonrpc": "2.0",
            "id": 5,
            "method": "tools/call",
            "params": {
                "name": "squeeze",
                "arguments": {
                    "filePath": file_path,
                    "queryPattern": "route_context",
                    "nodeType": "function"
                }
            }
        });
        let response = response_value(&request.to_string())?;
        let text = response["result"]["content"][0]["text"]
            .as_str()
            .ok_or("missing text content")?;
        let squeezed: Value = serde_json::from_str(text)?;

        assert_eq!(response["id"], 5);
        assert_eq!(squeezed["nodeType"], "function_item");
        assert_eq!(squeezed["startLine"], 3);
        assert_eq!(
            squeezed["text"],
            "pub fn route_context() {\n    println!(\"ok\");\n}"
        );

        Ok(())
    }

    #[test]
    fn calls_patch() -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        let file_path = root.path().join("lib.rs");
        fs::write(&file_path, "fn main() {\n    old_name();\n}\n")?;

        let request = json!({
            "jsonrpc": "2.0",
            "id": 6,
            "method": "tools/call",
            "params": {
                "name": "patch",
                "arguments": {
                    "filePath": file_path,
                    "startLine": 2,
                    "startColumn": 5,
                    "endLine": 2,
                    "endColumn": 13,
                    "replacement": "new_name"
                }
            }
        });
        let response = response_value(&request.to_string())?;
        let text = response["result"]["content"][0]["text"]
            .as_str()
            .ok_or("missing text content")?;
        let patch: Value = serde_json::from_str(text)?;

        assert_eq!(response["id"], 6);
        assert_eq!(patch["syntaxValid"], true);
        assert_eq!(
            fs::read_to_string(root.path().join("lib.rs"))?,
            "fn main() {\n    new_name();\n}\n"
        );

        Ok(())
    }
}
