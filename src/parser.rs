use tree_sitter::{Node, Parser, Tree};

use crate::walker::SourceKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedFunction {
    pub name: String,
    pub signature: String,
    pub line: usize,
}

pub struct TsExportParser {
    ts_parser: Parser,
    tsx_parser: Parser,
}

impl TsExportParser {
    pub fn new() -> Result<Self, String> {
        let mut ts_parser = Parser::new();
        ts_parser
            .set_language(&tree_sitter_typescript::language_typescript())
            .map_err(|err| format!("{err}"))?;

        let mut tsx_parser = Parser::new();
        tsx_parser
            .set_language(&tree_sitter_typescript::language_tsx())
            .map_err(|err| format!("{err}"))?;

        Ok(Self {
            ts_parser,
            tsx_parser,
        })
    }

    pub fn extract_exports_for_source(
        &mut self,
        source: &str,
        kind: &SourceKind,
    ) -> Result<Vec<ExtractedFunction>, String> {
        match kind {
            SourceKind::Ts => self.extract_exports_from_ts(source),
            SourceKind::Tsx => self.extract_exports_from_tsx(source),
            SourceKind::Vue => self.extract_exports_from_vue(source),
        }
    }

    fn extract_exports_from_ts(&mut self, source: &str) -> Result<Vec<ExtractedFunction>, String> {
        parse_with(&mut self.ts_parser, source)
    }

    fn extract_exports_from_tsx(&mut self, source: &str) -> Result<Vec<ExtractedFunction>, String> {
        parse_with(&mut self.tsx_parser, source)
    }

    fn extract_exports_from_vue(&mut self, source: &str) -> Result<Vec<ExtractedFunction>, String> {
        let blocks = extract_vue_scripts(source);
        let mut all_exports = Vec::new();

        for block in blocks {
            let mut exports = match block.kind {
                SourceKind::Tsx => self.extract_exports_from_tsx(&block.content),
                _ => self.extract_exports_from_ts(&block.content),
            }?;

            for export in &mut exports {
                export.line += block.line_offset;
            }

            all_exports.extend(exports);
        }

        all_exports.sort_by(|a, b| a.line.cmp(&b.line).then(a.name.cmp(&b.name)));
        Ok(all_exports)
    }
}

#[derive(Debug, Clone)]
struct VueScriptBlock {
    content: String,
    line_offset: usize,
    kind: SourceKind,
}

fn extract_vue_scripts(source: &str) -> Vec<VueScriptBlock> {
    let lower = source.to_ascii_lowercase();
    let mut out = Vec::new();
    let mut search_from = 0usize;

    while let Some(tag_rel) = lower[search_from..].find("<script") {
        let tag_start = search_from + tag_rel;
        let Some(tag_end_rel) = lower[tag_start..].find('>') else {
            break;
        };
        let tag_end = tag_start + tag_end_rel;
        let attrs = &lower[tag_start + "<script".len()..tag_end];

        let Some(close_rel) = lower[tag_end + 1..].find("</script>") else {
            break;
        };
        let close_start = tag_end + 1 + close_rel;
        let content_start = tag_end + 1;
        let content_end = close_start;

        search_from = close_start + "</script>".len();

        if attrs.contains("src=") {
            continue;
        }

        let kind = if attrs.contains("lang=\"tsx\"")
            || attrs.contains("lang='tsx'")
            || attrs.contains("lang=tsx")
        {
            SourceKind::Tsx
        } else {
            SourceKind::Ts
        };

        let line_offset = source[..content_start].bytes().filter(|b| *b == b'\n').count();
        let content = source[content_start..content_end].to_string();

        out.push(VueScriptBlock {
            content,
            line_offset,
            kind,
        });
    }

    out
}

fn parse_with(parser: &mut Parser, source: &str) -> Result<Vec<ExtractedFunction>, String> {
    let tree = parser
        .parse(source, None)
        .ok_or_else(|| "failed to parse file".to_string())?;

    if tree.root_node().has_error() {
        return Err("syntax parse error".to_string());
    }

    Ok(extract_from_tree(&tree, source))
}

fn extract_from_tree(tree: &Tree, source: &str) -> Vec<ExtractedFunction> {
    let mut exports = Vec::new();
    let root = tree.root_node();
    let mut cursor = root.walk();

    for child in root.children(&mut cursor) {
        if child.kind() != "export_statement" {
            continue;
        }

        let Some(exported) = first_named_child(child, source) else {
            continue;
        };

        match exported.kind() {
            "function_declaration" => {
                if let Some(extracted) = function_declaration_export(exported, source) {
                    exports.push(extracted);
                }
            }
            "lexical_declaration" => {
                if is_const_lexical(exported, source) {
                    exports.extend(const_callable_exports(exported, source));
                }
            }
            _ => {}
        }
    }

    exports.sort_by(|a, b| a.line.cmp(&b.line).then(a.name.cmp(&b.name)));
    exports
}

fn first_named_child<'a>(node: Node<'a>, source: &str) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).find(|child| {
        let kind = child.kind();
        if kind == "export_clause" || kind == "namespace_export" {
            return false;
        }

        let text = text_for(*child, source);
        !text.starts_with("type ")
    })
}

fn function_declaration_export(node: Node<'_>, source: &str) -> Option<ExtractedFunction> {
    let name_node = node.child_by_field_name("name")?;
    let name = text_for(name_node, source).to_string();
    let parameters = node
        .child_by_field_name("parameters")
        .map(|n| text_for(n, source).to_string())
        .unwrap_or_else(|| "()".to_string());
    let return_type = node
        .child_by_field_name("return_type")
        .map(|n| text_for(n, source).trim().to_string())
        .unwrap_or_default();

    let signature = if return_type.is_empty() {
        format!("{name}{parameters}")
    } else {
        format!("{name}{parameters} {return_type}")
    };

    Some(ExtractedFunction {
        name,
        signature,
        line: name_node.start_position().row + 1,
    })
}

fn is_const_lexical(node: Node<'_>, source: &str) -> bool {
    let mut cursor = node.walk();
    let mut children = node.children(&mut cursor);
    if let Some(first) = children.next() {
        return text_for(first, source).trim() == "const";
    }

    false
}

fn const_callable_exports(node: Node<'_>, source: &str) -> Vec<ExtractedFunction> {
    let mut out = Vec::new();
    let mut cursor = node.walk();

    for declarator in node
        .named_children(&mut cursor)
        .filter(|child| child.kind() == "variable_declarator")
    {
        let Some(name_node) = declarator.child_by_field_name("name") else {
            continue;
        };

        if name_node.kind() != "identifier" {
            continue;
        }

        let Some(value_node) = declarator.child_by_field_name("value") else {
            continue;
        };

        let name = text_for(name_node, source).to_string();

        match value_node.kind() {
            "arrow_function" => {
                out.push(build_from_arrow(name, name_node, value_node, source));
            }
            "function" => {
                out.push(build_from_function_expr(name, name_node, value_node, source));
            }
            _ => {}
        }
    }

    out
}

fn build_from_arrow(
    name: String,
    name_node: Node<'_>,
    node: Node<'_>,
    source: &str,
) -> ExtractedFunction {
    let raw_params = node
        .child_by_field_name("parameters")
        .or_else(|| node.child_by_field_name("parameter"))
        .map(|n| text_for(n, source).to_string())
        .unwrap_or_else(|| "()".to_string());

    let parameters = if raw_params.trim_start().starts_with('(') {
        raw_params
    } else {
        format!("({raw_params})")
    };

    let return_type = node
        .child_by_field_name("return_type")
        .map(|n| text_for(n, source).trim().to_string())
        .unwrap_or_default();

    let signature = if return_type.is_empty() {
        format!("{name}{parameters}")
    } else {
        format!("{name}{parameters} {return_type}")
    };

    ExtractedFunction {
        name,
        signature,
        line: name_node.start_position().row + 1,
    }
}

fn build_from_function_expr(
    name: String,
    name_node: Node<'_>,
    node: Node<'_>,
    source: &str,
) -> ExtractedFunction {
    let parameters = node
        .child_by_field_name("parameters")
        .map(|n| text_for(n, source).to_string())
        .unwrap_or_else(|| "()".to_string());
    let return_type = node
        .child_by_field_name("return_type")
        .map(|n| text_for(n, source).trim().to_string())
        .unwrap_or_default();

    let signature = if return_type.is_empty() {
        format!("{name}{parameters}")
    } else {
        format!("{name}{parameters} {return_type}")
    };

    ExtractedFunction {
        name,
        signature,
        line: name_node.start_position().row + 1,
    }
}

fn text_for<'a>(node: Node<'_>, source: &'a str) -> &'a str {
    let range = node.byte_range();
    &source[range]
}

#[cfg(test)]
mod tests {
    use crate::walker::SourceKind;

    use super::TsExportParser;

    #[test]
    fn detects_exported_function_declaration() {
        let mut parser = TsExportParser::new().expect("parser");
        let source = "export function greet(name: string): string { return name }";
        let exports = parser
            .extract_exports_for_source(source, &SourceKind::Ts)
            .expect("extract");

        assert_eq!(exports.len(), 1);
        assert_eq!(exports[0].name, "greet");
        assert_eq!(exports[0].signature, "greet(name: string) : string");
    }

    #[test]
    fn detects_exported_const_arrow_function() {
        let mut parser = TsExportParser::new().expect("parser");
        let source = "export const sum = (a: number, b: number): number => a + b;";
        let exports = parser
            .extract_exports_for_source(source, &SourceKind::Ts)
            .expect("extract");

        assert_eq!(exports.len(), 1);
        assert_eq!(exports[0].name, "sum");
        assert_eq!(exports[0].signature, "sum(a: number, b: number) : number");
    }

    #[test]
    fn detects_exported_tsx_callable() {
        let mut parser = TsExportParser::new().expect("parser");
        let source = "export const Render = (name: string) => <div>{name}</div>;";
        let exports = parser
            .extract_exports_for_source(source, &SourceKind::Tsx)
            .expect("extract");

        assert_eq!(exports.len(), 1);
        assert_eq!(exports[0].name, "Render");
        assert_eq!(exports[0].signature, "Render(name: string)");
    }

    #[test]
    fn detects_exported_functions_in_vue_script() {
        let mut parser = TsExportParser::new().expect("parser");
        let source = r#"
<template><div /></template>
<script lang="ts">
export function fromVue(input: string): string {
  return input;
}
</script>
"#;
        let exports = parser
            .extract_exports_for_source(source, &SourceKind::Vue)
            .expect("extract");

        assert_eq!(exports.len(), 1);
        assert_eq!(exports[0].name, "fromVue");
        assert_eq!(exports[0].line, 4);
    }

    #[test]
    fn ignores_non_exported_and_reexports() {
        let mut parser = TsExportParser::new().expect("parser");
        let source = r#"
function internalFn() {}
export { internalFn }
export { externalFn } from "./dep"
"#;
        let exports = parser
            .extract_exports_for_source(source, &SourceKind::Ts)
            .expect("extract");

        assert!(exports.is_empty());
    }
}
