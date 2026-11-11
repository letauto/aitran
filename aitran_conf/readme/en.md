# aitran_conf

NestedText task configuration loader for `aii18n.nt`.

## Install

```toml
[dependencies]
aitran_conf = "0.1"
```

## Format

```text
-
  dir: ./i18n
  from: zh
  to:
    - en
```

| Field | Description |
| ----- | ----------- |
| `dir` | Source directory, required |
| `from` | Source language, required |
| `to` | Target languages; empty expands to every language except the source |

Language codes are parsed by `aitran_lang::by_code`; matching is case-insensitive and `-` equals `_`.

## Usage

```rust
use aitran_conf::from_str;

let conf = from_str(
  "-\n  dir: ./i18n\n  from: zh\n  to:\n    - en\n",
)?.unwrap();

assert_eq!(conf.task[0].from_name(), "简体中文");
assert_eq!(conf.task[0].to_name().collect::<Vec<_>>(), ["English"]);
```

## API

| Item | Description |
| ---- | ----------- |
| `from_str(text)` | Parse task config; empty document returns `None` |
| `load_from(path)` | Read and parse; missing file returns `None` |
| `parse_as::<T>(text)` | Parse any NestedText structure |
| `load_as::<T>(path)` | Read and parse any NestedText structure |
