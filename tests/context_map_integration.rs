use std::fs;

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
        "export const sum = (a: number, b: number): number => a + b;\n",
    )
    .expect("write callable");
    fs::write(
        root.join("src/component.tsx"),
        "export const Render = (label: string) => <span>{label}</span>;\n",
    )
    .expect("write tsx");
    fs::write(
        root.join("src/view.vue"),
        "<template><div /></template>\n<script lang=\"ts\">\nexport function fromVue(id: string): string { return id }\n</script>\n",
    )
    .expect("write vue");

    fs::write(root.join("src/invalid.ts"), "export function bad( {\n").expect("write invalid");

    fs::write(root.join("dist/ignored.ts"), "export function ignored() {}\n")
        .expect("write ignored");

    let result = context_map::generate_context_map(root).expect("generate");

    assert_eq!(result.summary.scanned, 5);
    assert_eq!(result.summary.parsed, 4);
    assert_eq!(result.summary.parse_failed, 1);
    assert_eq!(result.summary.exported_functions, 4);

    let md = context_map::markdown::render_markdown(&result);
    assert!(md.contains("hello(name: string) : string"));
    assert!(md.contains("sum(a: number, b: number) : number"));
    assert!(md.contains("Render(label: string)"));
    assert!(md.contains("fromVue(id: string) : string"));
    assert!(md.contains("## Parse Errors"));
    assert!(!md.contains("ignored.ts"));
}
