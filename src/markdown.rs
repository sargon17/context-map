use std::collections::BTreeMap;

use crate::{RepoEntry, RunOutput};

#[derive(Default)]
struct TreeNode {
    is_dir: bool,
    children: BTreeMap<String, TreeNode>,
}

pub fn render_markdown(output: &RunOutput) -> String {
    let mut lines = Vec::new();

    lines.push("# Repository Structure".to_string());
    lines.push("```text".to_string());
    lines.extend(render_repo_tree(&output.repo_entries));
    lines.push("```".to_string());
    lines.push(String::new());

    lines.push("# Exported Functions".to_string());

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

fn render_repo_tree(entries: &[RepoEntry]) -> Vec<String> {
    if entries.is_empty() {
        return vec![".".to_string()];
    }

    let mut root = TreeNode {
        is_dir: true,
        children: BTreeMap::new(),
    };

    for entry in entries {
        let parts = entry
            .path
            .split('/')
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>();

        if parts.is_empty() {
            continue;
        }

        let mut current = &mut root;
        for (idx, part) in parts.iter().enumerate() {
            let is_last = idx == parts.len() - 1;
            current = current
                .children
                .entry((*part).to_string())
                .or_insert_with(|| TreeNode {
                    is_dir: !is_last || entry.is_dir,
                    children: BTreeMap::new(),
                });

            if is_last {
                current.is_dir = entry.is_dir;
            }
        }
    }

    let mut lines = vec![".".to_string()];
    render_children(&root, "", &mut lines);
    lines
}

fn render_children(node: &TreeNode, prefix: &str, out: &mut Vec<String>) {
    let mut children = node.children.iter().collect::<Vec<_>>();
    children.sort_by(|(a_name, a_node), (b_name, b_node)| {
        b_node
            .is_dir
            .cmp(&a_node.is_dir)
            .then(a_name.cmp(b_name))
    });

    let last_index = children.len().saturating_sub(1);

    for (idx, (name, child)) in children.into_iter().enumerate() {
        let is_last = idx == last_index;
        let branch = if is_last { "└──" } else { "├──" };
        out.push(format!("{prefix}{branch} {name}"));

        let next_prefix = if is_last {
            format!("{prefix}    ")
        } else {
            format!("{prefix}│   ")
        };

        if !child.children.is_empty() {
            render_children(child, &next_prefix, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{FileResult, FunctionExport, RepoEntry, RunOutput, RunSummary};

    use super::render_markdown;

    #[test]
    fn renders_grouped_sorted_markdown() {
        let output = RunOutput {
            root_path: "/tmp/repo".to_string(),
            repo_entries: vec![
                RepoEntry {
                    path: "Cargo.toml".to_string(),
                    is_dir: false,
                    depth: 1,
                },
                RepoEntry {
                    path: "src".to_string(),
                    is_dir: true,
                    depth: 1,
                },
                RepoEntry {
                    path: "src/a.ts".to_string(),
                    is_dir: false,
                    depth: 2,
                },
            ],
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

        assert!(markdown.starts_with("# Repository Structure\n```text\n.\n"));
        assert!(markdown.contains("├── src"));
        assert!(markdown.contains("└── Cargo.toml"));
        assert!(markdown.contains("# Exported Functions"));
        assert!(markdown.contains("### `src/a.ts`"));
        assert!(markdown.contains("### `src/b.ts`"));
        assert!(markdown.contains("`a(): string` (`src/a.ts:2`)"));
        assert!(markdown.contains("`src/c.ts`: syntax parse error"));
    }
}
