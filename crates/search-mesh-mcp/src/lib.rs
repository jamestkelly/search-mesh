use std::{io, path::PathBuf};

use search_mesh_core::{ScanMatch, ScanRequest, scan_keywords};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;

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
struct PulseHyperScanArgs {
    target_dirs: Vec<PathBuf>,
    keywords: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanMatchPayload {
    file: String,
    line: usize,
    keyword: String,
    match_str: String,
}

pub fn handle_jsonrpc(input: &str) -> String {
    let response = match serde_json::from_str::<JsonRpcRequest>(input) {
        Ok(request) => dispatch(request),
        Err(error) => jsonrpc_error(None, -32700, format!("parse error: {error}")),
    };

    response.to_string()
}

fn dispatch(request: JsonRpcRequest) -> Value {
    match request.method.as_str() {
        "tools/list" => jsonrpc_result(request.id, tools_list()),
        "tools/call" => dispatch_tool_call(request.id, request.params),
        method => jsonrpc_error(
            request.id,
            -32601,
            format!("unsupported JSON-RPC method: {method}"),
        ),
    }
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
        name => jsonrpc_error(id, -32602, format!("unsupported tool: {name}")),
    }
}

fn call_scan(id: Option<Value>, arguments: Value) -> Value {
    let arguments = match serde_json::from_value::<PulseHyperScanArgs>(arguments) {
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

fn scan_matches_payload(matches: Vec<ScanMatch>) -> Vec<ScanMatchPayload> {
    matches
        .into_iter()
        .map(|scan_match| ScanMatchPayload {
            file: scan_match.file.display().to_string(),
            line: scan_match.line,
            keyword: scan_match.keyword,
            match_str: scan_match.match_str,
        })
        .collect()
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

    fn response_value(input: &str) -> serde_json::Result<Value> {
        serde_json::from_str(&handle_jsonrpc(input))
    }

    #[test]
    fn lists_scan_tool() -> serde_json::Result<()> {
        let response = response_value(r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#)?;

        assert_eq!(response["jsonrpc"], "2.0");
        assert_eq!(response["id"], 1);
        assert_eq!(response["result"]["tools"][0]["name"], "scan");

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
    fn rejects_unknown_tool() -> serde_json::Result<()> {
        let response = response_value(
            r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"missing_tool"}}"#,
        )?;

        assert_eq!(response["error"]["code"], -32602);

        Ok(())
    }
}
