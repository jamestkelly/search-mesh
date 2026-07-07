use std::{
    fs,
    path::{Path, PathBuf},
};

use thiserror::Error;
use tree_sitter::{Language, Node, Parser};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeRequest {
    pub file_path: PathBuf,
    pub query_pattern: String,
    pub node_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeResponse {
    pub is_valid: bool,
    pub node_type: Option<String>,
    pub start_line: Option<usize>,
    pub end_line: Option<usize>,
}

#[derive(Debug, Error)]
pub enum ProbeError {
    #[error("file path is required")]
    MissingFilePath,
    #[error("query pattern is required")]
    MissingQueryPattern,
    #[error("node type is required")]
    MissingNodeType,
    #[error("unsupported file extension: {0}")]
    UnsupportedFileExtension(String),
    #[error("failed to read source file: {0}")]
    ReadSource(#[from] std::io::Error),
    #[error("failed to set parser language: {0}")]
    Language(#[from] tree_sitter::LanguageError),
    #[error("failed to parse source file")]
    Parse,
}

pub fn ast_probe(request: &ProbeRequest) -> Result<ProbeResponse, ProbeError> {
    if request.file_path.as_os_str().is_empty() {
        return Err(ProbeError::MissingFilePath);
    }

    if request.query_pattern.is_empty() {
        return Err(ProbeError::MissingQueryPattern);
    }

    if request.node_type.is_empty() {
        return Err(ProbeError::MissingNodeType);
    }

    let source = fs::read_to_string(&request.file_path)?;
    let language = language_for_path(&request.file_path)?;
    let target_node_kinds = node_kinds_for_alias(language, &request.node_type);
    let mut parser = Parser::new();
    parser.set_language(&language.parser_language())?;
    let tree = parser.parse(&source, None).ok_or(ProbeError::Parse)?;
    for (match_start, _) in source.match_indices(&request.query_pattern) {
        let match_end = match_start + request.query_pattern.len();
        if let Some(node) = find_covering_node(tree.root_node(), match_start, match_end) {
            let mut current = Some(node);
            while let Some(candidate) = current {
                if target_node_kinds
                    .iter()
                    .any(|kind| kind == candidate.kind())
                {
                    return Ok(valid_response(candidate));
                }
                current = candidate.parent();
            }
        }
    }

    Ok(ProbeResponse {
        is_valid: false,
        node_type: None,
        start_line: None,
        end_line: None,
    })
}

fn valid_response(node: Node<'_>) -> ProbeResponse {
    ProbeResponse {
        is_valid: true,
        node_type: Some(node.kind().to_string()),
        start_line: Some(node.start_position().row + 1),
        end_line: Some(node.end_position().row + 1),
    }
}

fn find_covering_node(node: Node<'_>, start_byte: usize, end_byte: usize) -> Option<Node<'_>> {
    if node.start_byte() > start_byte || node.end_byte() < end_byte {
        return None;
    }

    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(match_node) = find_covering_node(child, start_byte, end_byte) {
            return Some(match_node);
        }
    }

    Some(node)
}

#[derive(Debug, Clone, Copy)]
enum SupportedLanguage {
    Rust,
    Python,
    JavaScript,
    TypeScript,
}

impl SupportedLanguage {
    fn parser_language(self) -> Language {
        match self {
            Self::Rust => tree_sitter_rust::LANGUAGE.into(),
            Self::Python => tree_sitter_python::LANGUAGE.into(),
            Self::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
            Self::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        }
    }
}

fn language_for_path(path: &Path) -> Result<SupportedLanguage, ProbeError> {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .unwrap_or_default();

    match extension {
        "rs" => Ok(SupportedLanguage::Rust),
        "py" => Ok(SupportedLanguage::Python),
        "js" | "jsx" | "mjs" | "cjs" => Ok(SupportedLanguage::JavaScript),
        "ts" | "tsx" => Ok(SupportedLanguage::TypeScript),
        extension => Err(ProbeError::UnsupportedFileExtension(extension.to_string())),
    }
}

fn node_kinds_for_alias(language: SupportedLanguage, alias: &str) -> Vec<String> {
    match (language, alias) {
        (SupportedLanguage::Rust, "function") => vec!["function_item".to_string()],
        (SupportedLanguage::Rust, "struct") => vec!["struct_item".to_string()],
        (SupportedLanguage::Rust, "impl") => vec!["impl_item".to_string()],
        (SupportedLanguage::Rust, "enum") => vec!["enum_item".to_string()],
        (SupportedLanguage::Python, "function") => vec!["function_definition".to_string()],
        (SupportedLanguage::Python, "class") => vec!["class_definition".to_string()],
        (SupportedLanguage::JavaScript, "function") => {
            vec![
                "function_declaration".to_string(),
                "method_definition".to_string(),
            ]
        }
        (SupportedLanguage::JavaScript, "class") => vec!["class_declaration".to_string()],
        (SupportedLanguage::TypeScript, "function") => {
            vec![
                "function_declaration".to_string(),
                "method_definition".to_string(),
            ]
        }
        (SupportedLanguage::TypeScript, "class") => vec!["class_declaration".to_string()],
        (SupportedLanguage::TypeScript, "interface") => vec!["interface_declaration".to_string()],
        _ => vec![alias.to_string()],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, io};

    use tempfile::TempDir;

    fn write_file(root: &TempDir, path: &str, content: &str) -> io::Result<PathBuf> {
        let file_path = root.path().join(path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&file_path, content)?;
        Ok(file_path)
    }

    #[test]
    fn validates_rust_function() -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        let file_path = write_file(
            &root,
            "lib.rs",
            "pub fn scan_keywords() {\n    println!(\"ok\");\n}\n",
        )?;

        let response = ast_probe(&ProbeRequest {
            file_path,
            query_pattern: "scan_keywords".to_string(),
            node_type: "function".to_string(),
        })?;

        assert_eq!(response.node_type.as_deref(), Some("function_item"));
        assert_eq!(response.start_line, Some(1));
        assert_eq!(response.end_line, Some(3));
        assert!(response.is_valid);

        Ok(())
    }

    #[test]
    fn validates_python_class() -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        let file_path = write_file(&root, "model.py", "class SearchMesh:\n    pass\n")?;

        let response = ast_probe(&ProbeRequest {
            file_path,
            query_pattern: "SearchMesh".to_string(),
            node_type: "class".to_string(),
        })?;

        assert_eq!(response.node_type.as_deref(), Some("class_definition"));
        assert!(response.is_valid);

        Ok(())
    }

    #[test]
    fn validates_typescript_function() -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        let file_path = write_file(
            &root,
            "index.ts",
            "function routeContext() {\n  return true;\n}\n",
        )?;

        let response = ast_probe(&ProbeRequest {
            file_path,
            query_pattern: "routeContext".to_string(),
            node_type: "function".to_string(),
        })?;

        assert_eq!(response.node_type.as_deref(), Some("function_declaration"));
        assert!(response.is_valid);

        Ok(())
    }

    #[test]
    fn validates_javascript_function() -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        let file_path = write_file(
            &root,
            "index.js",
            "function routeContext() {\n  return true;\n}\n",
        )?;

        let response = ast_probe(&ProbeRequest {
            file_path,
            query_pattern: "routeContext".to_string(),
            node_type: "function".to_string(),
        })?;

        assert_eq!(response.node_type.as_deref(), Some("function_declaration"));
        assert!(response.is_valid);

        Ok(())
    }

    #[test]
    fn rejects_wrong_node_type() -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        let file_path = write_file(&root, "lib.rs", "struct SearchMesh;\n")?;

        let response = ast_probe(&ProbeRequest {
            file_path,
            query_pattern: "SearchMesh".to_string(),
            node_type: "function".to_string(),
        })?;

        assert!(!response.is_valid);
        assert_eq!(response.node_type, None);

        Ok(())
    }
}
