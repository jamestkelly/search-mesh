use std::{fs, path::PathBuf};

use aho_corasick::AhoCorasick;
use ignore::WalkBuilder;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanRequest {
    pub target_dirs: Vec<PathBuf>,
    pub keywords: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanMatch {
    pub file: PathBuf,
    pub line: usize,
    pub keyword: String,
    pub match_str: String,
}

#[derive(Debug, Error)]
pub enum ScanError {
    #[error("at least one target directory is required")]
    MissingTargetDirs,
    #[error("at least one keyword is required")]
    MissingKeywords,
    #[error("failed to build keyword matcher: {0}")]
    Matcher(#[from] aho_corasick::BuildError),
}

pub fn scan_keywords(request: &ScanRequest) -> Result<Vec<ScanMatch>, ScanError> {
    if request.target_dirs.is_empty() {
        return Err(ScanError::MissingTargetDirs);
    }

    if request.keywords.is_empty() {
        return Err(ScanError::MissingKeywords);
    }

    let matcher = AhoCorasick::new(&request.keywords)?;
    let mut matches = Vec::new();

    for target_dir in &request.target_dirs {
        let mut walker = WalkBuilder::new(target_dir);
        walker.require_git(false);

        for entry in walker.build().filter_map(Result::ok) {
            if !entry
                .file_type()
                .is_some_and(|file_type| file_type.is_file())
            {
                continue;
            }

            let path = entry.path();
            let Ok(content) = fs::read_to_string(path) else {
                continue;
            };

            for (line_index, line) in content.lines().enumerate() {
                for mat in matcher.find_iter(line) {
                    matches.push(ScanMatch {
                        file: path.to_path_buf(),
                        line: line_index + 1,
                        keyword: request.keywords[mat.pattern()].clone(),
                        match_str: line.to_string(),
                    });
                }
            }
        }
    }

    Ok(matches)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    use tempfile::TempDir;

    fn write_file(root: &TempDir, path: &str, content: &str) -> io::Result<()> {
        let file_path = root.path().join(path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(file_path, content)
    }

    #[test]
    fn rejects_missing_target_dirs() {
        let request = ScanRequest {
            target_dirs: Vec::new(),
            keywords: vec!["TODO".to_string()],
        };

        assert!(matches!(
            scan_keywords(&request),
            Err(ScanError::MissingTargetDirs)
        ));
    }

    #[test]
    fn rejects_missing_keywords() {
        let request = ScanRequest {
            target_dirs: vec![PathBuf::from("src")],
            keywords: Vec::new(),
        };

        assert!(matches!(
            scan_keywords(&request),
            Err(ScanError::MissingKeywords)
        ));
    }

    #[test]
    fn finds_multiple_keywords_in_one_file() -> io::Result<()> {
        let root = TempDir::new()?;
        write_file(
            &root,
            "src/main.rs",
            "fn main() {\n    // TODO: remove deprecated path\n}\n",
        )?;

        let request = ScanRequest {
            target_dirs: vec![root.path().join("src")],
            keywords: vec!["TODO".to_string(), "deprecated".to_string()],
        };

        let matches = scan_keywords(&request).map_err(io::Error::other)?;

        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].line, 2);
        assert_eq!(matches[0].keyword, "TODO");
        assert_eq!(matches[0].match_str, "    // TODO: remove deprecated path");
        assert_eq!(matches[1].line, 2);
        assert_eq!(matches[1].keyword, "deprecated");

        Ok(())
    }

    #[test]
    fn scans_nested_files() -> io::Result<()> {
        let root = TempDir::new()?;
        write_file(&root, "src/lib.rs", "pub fn clean() {}\n")?;
        write_file(&root, "src/nested/module.rs", "// FIXME: split module\n")?;

        let request = ScanRequest {
            target_dirs: vec![root.path().join("src")],
            keywords: vec!["FIXME".to_string()],
        };

        let matches = scan_keywords(&request).map_err(io::Error::other)?;

        assert_eq!(matches.len(), 1);
        assert!(matches[0].file.ends_with("src/nested/module.rs"));
        assert_eq!(matches[0].line, 1);
        assert_eq!(matches[0].keyword, "FIXME");

        Ok(())
    }

    #[test]
    fn respects_gitignore_files() -> io::Result<()> {
        let root = TempDir::new()?;
        write_file(&root, ".gitignore", "ignored.rs\n")?;
        write_file(&root, "ignored.rs", "// TODO: hidden\n")?;
        write_file(&root, "visible.rs", "// TODO: visible\n")?;

        let request = ScanRequest {
            target_dirs: vec![root.path().to_path_buf()],
            keywords: vec!["TODO".to_string()],
        };

        let matches = scan_keywords(&request).map_err(io::Error::other)?;

        assert_eq!(matches.len(), 1);
        assert!(matches[0].file.ends_with("visible.rs"));
        assert_eq!(matches[0].match_str, "// TODO: visible");

        Ok(())
    }

    #[test]
    fn skips_non_utf8_files() -> io::Result<()> {
        let root = TempDir::new()?;
        fs::write(root.path().join("binary.bin"), [0xff, 0xfe, 0xfd])?;
        write_file(&root, "visible.rs", "// TODO: visible\n")?;

        let request = ScanRequest {
            target_dirs: vec![root.path().to_path_buf()],
            keywords: vec!["TODO".to_string()],
        };

        let matches = scan_keywords(&request).map_err(io::Error::other)?;

        assert_eq!(matches.len(), 1);
        assert!(matches[0].file.ends_with("visible.rs"));

        Ok(())
    }
}
