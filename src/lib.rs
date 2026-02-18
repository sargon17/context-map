use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs;
use std::path::{Path, PathBuf};

pub mod markdown;
pub mod parser;
pub mod walker;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionExport {
    pub name: String,
    pub signature: String,
    pub file_path: String,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileResult {
    pub file_path: String,
    pub exports: Vec<FunctionExport>,
    pub parse_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RunSummary {
    pub scanned: usize,
    pub parsed: usize,
    pub parse_failed: usize,
    pub exported_functions: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunOutput {
    pub root_path: String,
    pub summary: RunSummary,
    pub file_results: Vec<FileResult>,
}

#[derive(Debug)]
pub enum ContextMapError {
    InvalidRoot(PathBuf),
    ParserInit(String),
    Io(std::io::Error),
}

impl Display for ContextMapError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidRoot(path) => write!(f, "invalid root path: {}", path.display()),
            Self::ParserInit(msg) => write!(f, "failed to initialize parser: {msg}"),
            Self::Io(err) => write!(f, "io error: {err}"),
        }
    }
}

impl Error for ContextMapError {}

impl From<std::io::Error> for ContextMapError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

pub fn generate_context_map(root: &Path) -> Result<RunOutput, ContextMapError> {
    if !root.is_dir() {
        return Err(ContextMapError::InvalidRoot(root.to_path_buf()));
    }

    let canonical_root = fs::canonicalize(root)?;
    let mut ts_parser = parser::TsExportParser::new().map_err(ContextMapError::ParserInit)?;
    let files = walker::collect_source_files(&canonical_root)?;

    let mut summary = RunSummary {
        scanned: files.len(),
        ..RunSummary::default()
    };

    let mut file_results: Vec<FileResult> = Vec::with_capacity(files.len());

    for source_file in files {
        let relative = normalize_path(
            source_file
                .path
                .strip_prefix(&canonical_root)
                .unwrap_or(&source_file.path),
        );

        match fs::read_to_string(&source_file.path) {
            Ok(source) => match ts_parser.extract_exports_for_source(&source, &source_file.kind) {
                Ok(extracted) => {
                    summary.parsed += 1;
                    summary.exported_functions += extracted.len();
                    let exports = extracted
                        .into_iter()
                        .map(|entry| FunctionExport {
                            name: entry.name,
                            signature: entry.signature,
                            file_path: relative.clone(),
                            line: entry.line,
                        })
                        .collect::<Vec<_>>();

                    file_results.push(FileResult {
                        file_path: relative,
                        exports,
                        parse_error: None,
                    });
                }
                Err(err) => {
                    summary.parse_failed += 1;
                    file_results.push(FileResult {
                        file_path: relative,
                        exports: Vec::new(),
                        parse_error: Some(err),
                    });
                }
            },
            Err(err) => {
                summary.parse_failed += 1;
                file_results.push(FileResult {
                    file_path: relative,
                    exports: Vec::new(),
                    parse_error: Some(err.to_string()),
                });
            }
        }
    }

    file_results.sort_by(|a, b| a.file_path.cmp(&b.file_path));
    for file in &mut file_results {
        file.exports.sort_by(|a, b| a.line.cmp(&b.line).then(a.name.cmp(&b.name)));
    }

    Ok(RunOutput {
        root_path: canonical_root.display().to_string(),
        summary,
        file_results,
    })
}

pub fn run(root: &Path, out: &Path) -> Result<RunSummary, ContextMapError> {
    let output = generate_context_map(root)?;
    let markdown = markdown::render_markdown(&output);
    fs::write(out, markdown)?;
    Ok(output.summary)
}

pub fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}
