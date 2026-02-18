use std::collections::BTreeMap;

use crate::{RenderConfig, RenderProfile, RepoEntry, RunOutput};

#[derive(Default)]
struct TreeNode {
    is_dir: bool,
    children: BTreeMap<String, TreeNode>,
}

pub fn render_markdown(output: &RunOutput) -> String {
    render_markdown_with_config(output, RenderConfig::default())
}

pub fn render_markdown_with_config(output: &RunOutput, config: RenderConfig) -> String {
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
        .filter(|f| !f.function_exports.is_empty())
        .collect::<Vec<_>>();

    if files_with_exports.is_empty() {
        lines.push("No exported functions found.".to_string());
    } else {
        for file in files_with_exports {
            lines.push(String::new());
            lines.push(format!("### `{}`", file.file_path));
            for export in &file.function_exports {
                lines.push(format!(
                    "- `{}`",
                    format_function_entry(export, config.profile)
                ));
            }
        }
    }

    if config.include_types {
        lines.push(String::new());
        lines.push("# Type Inventory".to_string());
        let files_with_types = output
            .file_results
            .iter()
            .filter(|f| !f.type_exports.is_empty())
            .collect::<Vec<_>>();

        if files_with_types.is_empty() {
            lines.push("No exported types or interfaces found.".to_string());
        } else {
            for file in files_with_types {
                lines.push(String::new());
                lines.push(format!("### `{}`", file.file_path));
                for ty in &file.type_exports {
                    let value = match config.profile {
                        RenderProfile::Detailed => format!("{} @L{}", ty.name, ty.line),
                        _ => ty.name.clone(),
                    };
                    lines.push(format!("- `{value}`"));
                }
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

fn format_function_entry(export: &crate::FunctionExport, profile: RenderProfile) -> String {
    match profile {
        RenderProfile::Compact => export.name.clone(),
        RenderProfile::Balanced => {
            if let Some(params) = extract_parameters(&export.signature, &export.name) {
                format!("{}{}", export.name, normalize_whitespace(&params))
            } else {
                export.name.clone()
            }
        }
        RenderProfile::Detailed => format!("{} @L{}", normalize_whitespace(&export.signature), export.line),
    }
}

fn extract_parameters(signature: &str, name: &str) -> Option<String> {
    let rest = signature.strip_prefix(name)?.trim_start();
    let mut chars = rest.char_indices();
    let (start_idx, start_char) = chars.next()?;
    if start_idx != 0 || start_char != '(' {
        return None;
    }

    let mut depth = 0usize;
    for (idx, ch) in rest.char_indices() {
        if ch == '(' {
            depth += 1;
        } else if ch == ')' {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(rest[..=idx].to_string());
            }
        }
    }

    None
}

fn normalize_whitespace(input: &str) -> String {
    input.split_whitespace().collect::<Vec<_>>().join(" ")
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
    use crate::{
        FileResult, FunctionExport, RenderConfig, RenderProfile, RepoEntry, RunOutput, RunSummary,
        TypeExport,
    };

    use super::render_markdown_with_config;

    fn sample_output() -> RunOutput {
        RunOutput {
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
                exported_types: 1,
            },
            file_results: vec![
                FileResult {
                    file_path: "src/a.ts".to_string(),
                    function_exports: vec![FunctionExport {
                        name: "a".to_string(),
                        signature: "a(\n  x: number,\n  y: number,\n) : string".to_string(),
                        file_path: "src/a.ts".to_string(),
                        line: 2,
                    }],
                    type_exports: vec![TypeExport {
                        name: "User".to_string(),
                        file_path: "src/a.ts".to_string(),
                        line: 10,
                    }],
                    parse_error: None,
                },
                FileResult {
                    file_path: "src/c.ts".to_string(),
                    function_exports: vec![],
                    type_exports: vec![],
                    parse_error: Some("syntax parse error".to_string()),
                },
            ],
        }
    }

    #[test]
    fn compact_profile_is_token_lean() {
        let markdown = render_markdown_with_config(
            &sample_output(),
            RenderConfig {
                profile: RenderProfile::Compact,
                include_types: true,
                tree_depth: 10,
            },
        );

        assert!(markdown.contains("# Repository Structure"));
        assert!(markdown.contains("# Exported Functions"));
        assert!(markdown.contains("- `a`"));
        assert!(!markdown.contains("src/a.ts:2"));
        assert!(markdown.contains("# Type Inventory"));
        assert!(markdown.contains("- `User`"));
    }

    #[test]
    fn balanced_profile_compacts_signatures() {
        let markdown = render_markdown_with_config(
            &sample_output(),
            RenderConfig {
                profile: RenderProfile::Balanced,
                include_types: true,
                tree_depth: 10,
            },
        );

        assert!(markdown.contains("- `a( x: number, y: number, )`"));
        assert!(!markdown.contains("@L2"));
        assert!(!markdown.contains("src/a.ts:2"));
    }

    #[test]
    fn detailed_profile_adds_line_marker() {
        let markdown = render_markdown_with_config(
            &sample_output(),
            RenderConfig {
                profile: RenderProfile::Detailed,
                include_types: true,
                tree_depth: 10,
            },
        );

        assert!(markdown.contains("- `a( x: number, y: number, ) : string @L2`"));
        assert!(markdown.contains("- `User @L10`"));
    }

    #[test]
    fn can_disable_type_inventory() {
        let markdown = render_markdown_with_config(
            &sample_output(),
            RenderConfig {
                profile: RenderProfile::Balanced,
                include_types: false,
                tree_depth: 10,
            },
        );

        assert!(!markdown.contains("# Type Inventory"));
    }
}
