# context-map

`context-map` is a Rust CLI that scans a repository and generates a Markdown context file for coding agents.

It extracts:
- exported functions
- exported type aliases
- exported interfaces

from:
- `.ts`
- `.tsx`
- `.vue` (`<script>` blocks)

and writes a structured `context-map.md` with:
- repository tree
- exported functions grouped by file
- optional type inventory grouped by file
- parse error report

## Why this exists

Agents often duplicate code when they do not quickly see repo shape and existing symbols.

`context-map` gives a compact index that helps agents:
- navigate modules faster
- discover existing utilities/types
- reduce redundant implementations

## Features

- Syntax-aware parsing via Tree-sitter (no regex scraping)
- TS, TSX, Vue `<script>` support
- Grouped output by file path
- Token-efficient profiles (`compact`, `balanced`, `detailed`)
- Optional type inventory (`--no-types`)
- Configurable repository tree depth (`--tree-depth`)
- Parse errors are non-fatal and reported per file

## Installation / Build

Prerequisites:
- Rust toolchain (stable)

Build:

```bash
cargo build --release
```

Binary path:

```bash
./target/release/context-map
```

Run without building release:

```bash
cargo run -- --root .
```

## CLI Usage

```bash
context-map --root <path> [--out <file>] [--profile <compact|balanced|detailed>] [--no-types] [--tree-depth <N>]
```

### Options

- `--root <path>`
  - Root directory to scan
  - Default: `.`

- `--out <file>`
  - Output Markdown file path
  - Default: `<root>/context-map.md`

- `--profile <compact|balanced|detailed>`
  - Controls symbol formatting verbosity
  - Default: `balanced`

- `--no-types`
  - Disables the `Type Inventory` section

- `--tree-depth <N>`
  - Max depth for repository structure tree
  - Default: `10`

## Output Profiles

### `compact`
- Functions: name only
- Types: name only
- No line markers
- Best for lowest token usage

### `balanced` (default)
- Functions: `name(params)`
- Types: name only
- No line markers
- Best quality/token balance

### `detailed`
- Functions: full normalized signature + `@L<line>`
- Types: name + `@L<line>`
- Best when precise location detail is needed

## What is scanned

Source files considered for extraction:
- `*.ts`
- `*.tsx`
- `*.vue`

Ignored source files:
- `*.d.ts`
- `*.props.ts` (exact suffix rule)

Notes:
- `.props.tsx` is **not** ignored
- Vue `script src="..."` blocks are skipped

## Ignored directories

These directories are excluded from traversal:
- `.git`
- `node_modules`
- `dist`
- `build`
- `target`
- hidden nested directories (names starting with `.` below root)

## Extraction rules

### Exported functions
Included:
- `export function foo(...) {}`
- `export const foo = (...) => ...`
- `export const foo = function (...) {}`

### Exported types
Included:
- `export interface Foo { ... }`
- `export type Foo = ...`

### Not included
- Re-export lists/forwarding forms such as:
  - `export { foo }`
  - `export { foo } from "./x"`
  - `export * from "./x"`

## Output structure

Generated Markdown sections:
1. `# Repository Structure`
2. `# Exported Functions`
3. `# Type Inventory` (unless `--no-types`)
4. `## Parse Errors` (only when present)

Entries are grouped by file:

```md
### `src/utils/math.ts`
- `sum(a: number, b: number)`
- `average(values: number[])`
```

## Examples

Default (balanced):

```bash
cargo run -- --root /path/to/repo
```

Token-lean for agent prompts:

```bash
cargo run -- --root /path/to/repo --profile compact --no-types --tree-depth 4
```

Detailed audit mode:

```bash
cargo run -- --root /path/to/repo --profile detailed --tree-depth 12
```

Custom output file:

```bash
cargo run -- --root /path/to/repo --out /tmp/context-map.md
```

## Exit behavior

- Exit `0` on successful run (including when no exports are found)
- Non-zero on fatal errors (invalid root, parser init failure, output write failure)

Per-file parse/read failures are reported under `## Parse Errors` and do not fail the whole run.

## Development

Run tests:

```bash
cargo test
```

Project layout:
- `src/main.rs`: CLI argument parsing and command entrypoint
- `src/lib.rs`: orchestration, config, run pipeline
- `src/walker.rs`: file/repo traversal and ignore filtering
- `src/parser.rs`: Tree-sitter extraction for functions/types
- `src/markdown.rs`: Markdown rendering by profile
- `tests/context_map_integration.rs`: end-to-end integration checks

## Programmatic usage (library)

The crate exposes APIs used by the CLI:

- `generate_context_map(root: &Path)`
- `generate_context_map_with_depth(root: &Path, tree_depth: usize)`
- `run(root: &Path, out: &Path)`
- `run_with_config(root: &Path, out: &Path, config: RenderConfig)`

Core config:

- `RenderProfile::{Compact, Balanced, Detailed}`
- `RenderConfig { profile, include_types, tree_depth }`

## Current limitations

- Re-export resolution is intentionally out of scope
- Only TypeScript-family exports are indexed (TS/TSX/Vue script)
- Function extraction is declaration-based (not class/object method inventory)

## License

No license file is currently present in this repository.
