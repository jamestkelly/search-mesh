use std::{fs, path::PathBuf};

use thiserror::Error;

use crate::probe::{ProbeError, syntax_valid_for_path};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchRequest {
    pub file_path: PathBuf,
    pub start_line: usize,
    pub start_column: usize,
    pub end_line: usize,
    pub end_column: usize,
    pub replacement: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatchResponse {
    pub file: PathBuf,
    pub bytes_written: usize,
    pub syntax_valid: Option<bool>,
}

#[derive(Debug, Error)]
pub enum PatchError {
    #[error("file path is required")]
    MissingFilePath,
    #[error("line and column coordinates are 1-based")]
    ZeroCoordinate,
    #[error("patch range end is before start")]
    ReversedRange,
    #[error("line {0} is out of range")]
    LineOutOfRange(usize),
    #[error("column {column} is out of range on line {line}")]
    ColumnOutOfRange { line: usize, column: usize },
    #[error("column {column} on line {line} splits a UTF-8 codepoint")]
    InvalidUtf8Boundary { line: usize, column: usize },
    #[error("failed to read source file: {0}")]
    ReadSource(#[source] std::io::Error),
    #[error("failed to write source file: {0}")]
    WriteSource(#[source] std::io::Error),
    #[error(transparent)]
    Probe(#[from] ProbeError),
}

pub fn apply_patch(request: &PatchRequest) -> Result<PatchResponse, PatchError> {
    validate_request(request)?;

    let source = fs::read_to_string(&request.file_path).map_err(PatchError::ReadSource)?;
    let start = byte_offset(&source, request.start_line, request.start_column)?;
    let end = byte_offset(&source, request.end_line, request.end_column)?;

    if end < start {
        return Err(PatchError::ReversedRange);
    }

    let mut updated = source;
    updated.replace_range(start..end, &request.replacement);

    let syntax_valid = syntax_valid_for_path(&request.file_path, &updated)?;
    fs::write(&request.file_path, updated.as_bytes()).map_err(PatchError::WriteSource)?;

    Ok(PatchResponse {
        file: request.file_path.clone(),
        bytes_written: updated.len(),
        syntax_valid,
    })
}

fn validate_request(request: &PatchRequest) -> Result<(), PatchError> {
    if request.file_path.as_os_str().is_empty() {
        return Err(PatchError::MissingFilePath);
    }

    if request.start_line == 0
        || request.start_column == 0
        || request.end_line == 0
        || request.end_column == 0
    {
        return Err(PatchError::ZeroCoordinate);
    }

    Ok(())
}

fn byte_offset(source: &str, line: usize, column: usize) -> Result<usize, PatchError> {
    let line_start = line_start_offset(source, line)?;
    let line_end = line_end_offset(source, line_start);
    let column_offset = column - 1;
    let offset = line_start + column_offset;

    if offset > line_end {
        return Err(PatchError::ColumnOutOfRange { line, column });
    }

    if !source.is_char_boundary(offset) {
        return Err(PatchError::InvalidUtf8Boundary { line, column });
    }

    Ok(offset)
}

fn line_start_offset(source: &str, target_line: usize) -> Result<usize, PatchError> {
    if target_line == 1 {
        return Ok(0);
    }

    let mut current_line = 1;
    for (index, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            current_line += 1;
            if current_line == target_line {
                return Ok(index + 1);
            }
        }
    }

    Err(PatchError::LineOutOfRange(target_line))
}

fn line_end_offset(source: &str, line_start: usize) -> usize {
    source[line_start..].find('\n').map_or_else(
        || source.len(),
        |newline_offset| line_start + newline_offset,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

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
    fn replaces_text_by_line_and_column() -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        let file_path = write_file(&root, "lib.rs", "fn main() {\n    old_name();\n}\n")?;

        let response = apply_patch(&PatchRequest {
            file_path: file_path.clone(),
            start_line: 2,
            start_column: 5,
            end_line: 2,
            end_column: 13,
            replacement: "new_name".to_string(),
        })?;

        assert_eq!(
            fs::read_to_string(&file_path)?,
            "fn main() {\n    new_name();\n}\n"
        );
        assert_eq!(response.file, file_path);
        assert_eq!(response.syntax_valid, Some(true));

        Ok(())
    }

    #[test]
    fn reports_invalid_supported_syntax_after_patch() -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        let file_path = write_file(&root, "lib.rs", "fn main() {\n    old_name();\n}\n")?;

        let response = apply_patch(&PatchRequest {
            file_path: file_path.clone(),
            start_line: 1,
            start_column: 1,
            end_line: 1,
            end_column: 3,
            replacement: "".to_string(),
        })?;

        assert_eq!(response.syntax_valid, Some(false));

        Ok(())
    }

    #[test]
    fn syntax_valid_is_none_for_unsupported_files() -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        let file_path = write_file(&root, "note.txt", "hello old\n")?;

        let response = apply_patch(&PatchRequest {
            file_path,
            start_line: 1,
            start_column: 7,
            end_line: 1,
            end_column: 10,
            replacement: "new".to_string(),
        })?;

        assert_eq!(response.syntax_valid, None);

        Ok(())
    }

    #[test]
    fn rejects_utf8_boundary_splits() -> Result<(), Box<dyn std::error::Error>> {
        let root = TempDir::new()?;
        let file_path = write_file(&root, "note.txt", "é\n")?;

        let error = apply_patch(&PatchRequest {
            file_path,
            start_line: 1,
            start_column: 2,
            end_line: 1,
            end_column: 2,
            replacement: "x".to_string(),
        })
        .err()
        .ok_or("expected error")?;

        assert!(matches!(
            error,
            PatchError::InvalidUtf8Boundary { line: 1, column: 2 }
        ));

        Ok(())
    }
}
