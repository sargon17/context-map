use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};

use walkdir::{DirEntry, WalkDir};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceKind {
    Ts,
    Tsx,
    Vue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceFile {
    pub path: PathBuf,
    pub kind: SourceKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoEntry {
    pub path: PathBuf,
    pub is_dir: bool,
    pub depth: usize,
}

fn ignored_dirs() -> HashSet<&'static str> {
    [".git", "node_modules", "dist", "build", "target"]
        .into_iter()
        .collect()
}

fn should_descend(entry: &DirEntry) -> bool {
    if !entry.file_type().is_dir() {
        return true;
    }

    let name = entry.file_name().to_string_lossy();
    let ignored = ignored_dirs();

    if ignored.contains(name.as_ref()) {
        return false;
    }

    // Skip hidden tooling directories at any nested depth.
    if entry.depth() > 0 && name.starts_with('.') {
        return false;
    }

    true
}

fn classify_source_file(path: &Path) -> Option<SourceKind> {
    let ext = path.extension().and_then(|ext| ext.to_str())?;
    match ext {
        "ts" => {
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            if file_name.ends_with(".d.ts") {
                None
            } else {
                Some(SourceKind::Ts)
            }
        }
        "tsx" => Some(SourceKind::Tsx),
        "vue" => Some(SourceKind::Vue),
        _ => None,
    }
}

pub fn collect_source_files(root: &Path) -> io::Result<Vec<SourceFile>> {
    if !root.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("root is not a directory: {}", root.display()),
        ));
    }

    let mut files = WalkDir::new(root)
        .into_iter()
        .filter_entry(should_descend)
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| {
            let path = entry.path().to_path_buf();
            let kind = classify_source_file(&path)?;
            Some(SourceFile { path, kind })
        })
        .collect::<Vec<_>>();

    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

pub fn collect_repo_entries(root: &Path, max_depth: usize) -> io::Result<Vec<RepoEntry>> {
    if !root.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("root is not a directory: {}", root.display()),
        ));
    }

    let mut entries = WalkDir::new(root)
        .max_depth(max_depth)
        .into_iter()
        .filter_entry(should_descend)
        .filter_map(Result::ok)
        .filter(|entry| entry.depth() > 0)
        .map(|entry| RepoEntry {
            path: entry.path().to_path_buf(),
            is_dir: entry.file_type().is_dir(),
            depth: entry.depth(),
        })
        .collect::<Vec<_>>();

    entries.sort_by(|a, b| a.path.cmp(&b.path).then(a.is_dir.cmp(&b.is_dir).reverse()));
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::{collect_repo_entries, collect_source_files};

    #[test]
    fn skips_ignored_dirs_and_finds_nested_sources() {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path();

        fs::create_dir_all(root.join("src/nested")).expect("mkdir src");
        fs::create_dir_all(root.join("node_modules/pkg")).expect("mkdir node_modules");
        fs::create_dir_all(root.join(".git/hooks")).expect("mkdir .git");

        fs::write(root.join("src/index.ts"), "export function ok() {}\n").expect("write index");
        fs::write(root.join("src/view.tsx"), "export const Btn = () => <div />\n")
            .expect("write tsx");
        fs::write(root.join("src/comp.vue"), "<template/>\n<script>export function v() {}</script>\n")
            .expect("write vue");
        fs::write(root.join("src/nested/util.ts"), "export function nested() {}\n")
            .expect("write nested");
        fs::write(root.join("src/types.d.ts"), "declare const x: string\n")
            .expect("write dts");
        fs::write(root.join("node_modules/pkg/nope.ts"), "export function nope() {}\n")
            .expect("write ignored");

        let files = collect_source_files(root).expect("collect files");
        let paths = files
            .iter()
            .map(|p| {
                p.path
                    .strip_prefix(root)
                    .expect("relative")
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect::<Vec<_>>();

        assert_eq!(
            paths,
            vec!["src/comp.vue", "src/index.ts", "src/nested/util.ts", "src/view.tsx"]
        );
    }

    #[test]
    fn collects_repo_entries_with_depth_limit() {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path();

        fs::create_dir_all(root.join("a/b/c/d")).expect("mkdir deep");
        fs::create_dir_all(root.join("node_modules/pkg")).expect("mkdir ignored");
        fs::write(root.join("a/root.txt"), "ok\n").expect("write root");
        fs::write(root.join("a/b/c/inside.txt"), "ok\n").expect("write inside");
        fs::write(root.join("a/b/c/d/too-deep.txt"), "no\n").expect("write too deep");
        fs::write(root.join("node_modules/pkg/x.txt"), "no\n").expect("write ignored");

        let entries = collect_repo_entries(root, 3).expect("collect entries");
        let paths = entries
            .iter()
            .map(|e| {
                e.path
                    .strip_prefix(root)
                    .expect("relative")
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect::<Vec<_>>();

        assert!(paths.contains(&"a".to_string()));
        assert!(paths.contains(&"a/root.txt".to_string()));
        assert!(paths.contains(&"a/b".to_string()));
        assert!(paths.contains(&"a/b/c".to_string()));
        assert!(!paths.contains(&"a/b/c/inside.txt".to_string()));
        assert!(!paths.iter().any(|p| p.starts_with("node_modules")));
    }
}
