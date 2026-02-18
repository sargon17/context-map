use std::fs;

use context_map::{RenderConfig, RenderProfile};

#[test]
fn integration_handles_valid_and_invalid_files() {
    let temp = tempfile::tempdir().expect("tempdir");
    let root = temp.path();

    fs::create_dir_all(root.join("src")).expect("mkdir src");
    fs::create_dir_all(root.join("dist")).expect("mkdir dist");

    fs::write(
        root.join("src/valid.ts"),
        "export function hello(name: string): string { return name }\n",
    )
    .expect("write valid");

    fs::write(
        root.join("src/callable.ts"),
        "export const sum = (a: number, b: number): number => a + b;\nexport interface SumInput { a: number; b: number }\n",
    )
    .expect("write callable");
    fs::write(
        root.join("src/component.tsx"),
        "export const Render = (label: string) => <span>{label}</span>;\n",
    )
    .expect("write tsx");
    fs::write(
        root.join("src/view.vue"),
        "<template><div /></template>\n<script lang=\"ts\">\nexport function fromVue(id: string): string { return id }\nexport type VueId = string\n</script>\n",
    )
    .expect("write vue");

    fs::write(root.join("src/ignored.props.ts"), "export function ignoredProps() {}\n")
        .expect("write props ts");
    fs::write(root.join("src/kept.props.tsx"), "export const Kept = () => <div />;\n")
        .expect("write props tsx");

    fs::write(root.join("src/invalid.ts"), "export function bad( {\n").expect("write invalid");

    fs::write(root.join("dist/ignored.ts"), "export function ignored() {}\n")
        .expect("write ignored");

    let result = context_map::generate_context_map(root).expect("generate");

    assert_eq!(result.summary.scanned, 6);
    assert_eq!(result.summary.parsed, 5);
    assert_eq!(result.summary.parse_failed, 1);
    assert_eq!(result.summary.exported_functions, 5);
    assert_eq!(result.summary.exported_types, 2);

    let md_compact = context_map::markdown::render_markdown_with_config(
        &result,
        RenderConfig {
            profile: RenderProfile::Compact,
            include_types: true,
            tree_depth: 10,
        },
    );
    assert!(md_compact.contains("# Repository Structure"));
    assert!(md_compact.contains("- `hello`"));
    assert!(md_compact.contains("- `SumInput`"));
    assert!(!md_compact.contains("src/callable.ts:2"));
    assert!(!md_compact.contains("ignoredProps"));

    let md_balanced = context_map::markdown::render_markdown_with_config(
        &result,
        RenderConfig {
            profile: RenderProfile::Balanced,
            include_types: true,
            tree_depth: 10,
        },
    );
    assert!(md_balanced.contains("- `hello(name: string)`"));
    assert!(md_balanced.contains("- `sum(a: number, b: number)`"));
    assert!(!md_balanced.contains(": Promise"));
    assert!(!md_balanced.contains("src/callable.ts:2"));

    let md_detailed = context_map::markdown::render_markdown_with_config(
        &result,
        RenderConfig {
            profile: RenderProfile::Detailed,
            include_types: true,
            tree_depth: 10,
        },
    );
    assert!(md_detailed.contains("@L"));
    assert!(md_detailed.contains("- `VueId @L4`"));

    let md_no_types = context_map::markdown::render_markdown_with_config(
        &result,
        RenderConfig {
            profile: RenderProfile::Balanced,
            include_types: false,
            tree_depth: 10,
        },
    );
    assert!(!md_no_types.contains("# Type Inventory"));

    assert!(md_balanced.contains("## Parse Errors"));
    assert!(!md_balanced.contains("dist"));
}
