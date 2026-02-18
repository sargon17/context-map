use chrono::Utc;

use crate::RunOutput;

pub fn render_markdown(output: &RunOutput) -> String {
    let mut lines = Vec::new();

    lines.push("# Context Map".to_string());
    lines.push(String::new());
    lines.push(format!("Generated: {}", Utc::now().to_rfc3339()));
    lines.push(format!("Root: `{}`", output.root_path));
    lines.push(String::new());

    lines.push("## Summary".to_string());
    lines.push(format!("- Scanned `.ts` files: {}", output.summary.scanned));
    lines.push(format!("- Parsed files: {}", output.summary.parsed));
    lines.push(format!("- Parse errors: {}", output.summary.parse_failed));
    lines.push(format!(
        "- Exported functions: {}",
        output.summary.exported_functions
    ));
    lines.push(String::new());

    lines.push("## Exported Functions".to_string());

    let files_with_exports = output
        .file_results
        .iter()
        .filter(|f| !f.exports.is_empty())
        .collect::<Vec<_>>();

    if files_with_exports.is_empty() {
        lines.push("No exported functions found.".to_string());
    } else {
        for file in files_with_exports {
            lines.push(String::new());
            lines.push(format!("### `{}`", file.file_path));
            for export in &file.exports {
                lines.push(format!(
                    "- `{}` (`{}:{}`)",
                    export.signature, export.file_path, export.line
                ));
            }
        }
    }

    let parse_errors = output
        .file_results
        .iter()
        .filter_map(|f| f.parse_error.as_ref().map(|err| (&f.file_path, err)))
        .collect::<Vec<_>>();

    if !parse_errors.is_empty() {
        lines.push(String::new());
        lines.push("## Parse Errors".to_string());
        for (path, err) in parse_errors {
            lines.push(format!("- `{path}`: {err}"));
        }
    }

    lines.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    use crate::{FileResult, FunctionExport, RunOutput, RunSummary};

    use super::render_markdown;

    #[test]
    fn renders_grouped_sorted_markdown() {
        let output = RunOutput {
            root_path: "/tmp/repo".to_string(),
            summary: RunSummary {
                scanned: 3,
                parsed: 2,
                parse_failed: 1,
                exported_functions: 2,
            },
            file_results: vec![
                FileResult {
                    file_path: "src/a.ts".to_string(),
                    exports: vec![FunctionExport {
                        name: "a".to_string(),
                        signature: "a(): string".to_string(),
                        file_path: "src/a.ts".to_string(),
                        line: 2,
                    }],
                    parse_error: None,
                },
                FileResult {
                    file_path: "src/b.ts".to_string(),
                    exports: vec![FunctionExport {
                        name: "b".to_string(),
                        signature: "b(x: number)".to_string(),
                        file_path: "src/b.ts".to_string(),
                        line: 8,
                    }],
                    parse_error: None,
                },
                FileResult {
                    file_path: "src/c.ts".to_string(),
                    exports: vec![],
                    parse_error: Some("syntax parse error".to_string()),
                },
            ],
        };

        let markdown = render_markdown(&output);

        assert!(markdown.contains("## Summary"));
        assert!(markdown.contains("### `src/a.ts`"));
        assert!(markdown.contains("### `src/b.ts`"));
        assert!(markdown.contains("`a(): string` (`src/a.ts:2`)"));
        assert!(markdown.contains("`src/c.ts`: syntax parse error"));
    }
}
