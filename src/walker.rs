use std::collections::HashSet;
use std::io;
use std::path::{Path, PathBuf};

use walkdir::{DirEntry, WalkDir};

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

fn is_target_ts(path: &Path) -> bool {
    if path.extension().and_then(|ext| ext.to_str()) != Some("ts") {
        return false;
    }

    let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
    !file_name.ends_with(".d.ts")
}

pub fn collect_ts_files(root: &Path) -> io::Result<Vec<PathBuf>> {
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
        .map(|entry| entry.path().to_path_buf())
        .filter(|path| is_target_ts(path))
        .collect::<Vec<_>>();

    files.sort();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::collect_ts_files;

    #[test]
    fn skips_ignored_dirs_and_finds_nested_ts() {
        let temp = TempDir::new().expect("temp dir");
        let root = temp.path();

        fs::create_dir_all(root.join("src/nested")).expect("mkdir src");
        fs::create_dir_all(root.join("node_modules/pkg")).expect("mkdir node_modules");
        fs::create_dir_all(root.join(".git/hooks")).expect("mkdir .git");

        fs::write(root.join("src/index.ts"), "export function ok() {}\n").expect("write index");
        fs::write(root.join("src/nested/util.ts"), "export function nested() {}\n")
            .expect("write nested");
        fs::write(root.join("src/types.d.ts"), "declare const x: string\n")
            .expect("write dts");
        fs::write(root.join("node_modules/pkg/nope.ts"), "export function nope() {}\n")
            .expect("write ignored");

        let files = collect_ts_files(root).expect("collect files");
        let paths = files
            .iter()
            .map(|p| {
                p.strip_prefix(root)
                    .expect("relative")
                    .to_string_lossy()
                    .replace('\\', "/")
            })
            .collect::<Vec<_>>();

        assert_eq!(paths, vec!["src/index.ts", "src/nested/util.ts"]);
    }
}
