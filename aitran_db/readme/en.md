# aitran_db

Plain-text, diff-friendly i18n database: append-only, streaming compaction and hash lookup for previous sources.

## Install

```toml
[dependencies]
aitran_db = "0.1"
```

## Format

```text
@path/lang/<source hash>
>line
>line
```

- `@` starts a record header; the last two `/` split path, lang and hash
- `>` starts a content line
- Later records for the same `(path, lang)` win
- hash is the source file's BLAKE3 base64url, used to associate previous sources

## Usage

```rust
use aitran_db::Db;

let db = Db::open("i18n.db")?;
db.set("a.md", "zh", "hash", &["你好", "世界"]).await?;
let record = db.get_with_off("a.md", "zh").await?.unwrap();
assert_eq!(record.line_li, ["你好", "世界"]);

let old = db.get_hash("a.md", "zh", "hash").await?;
assert!(old.is_some());
```

## API

| Item | Description |
| ---- | ----------- |
| `Db::open(path)` | Open and acquire a process file lock |
| `set(path, lang, hash, lines)` | Append a record |
| `get(path, lang)` | Read latest content |
| `get_with_off(path, lang)` | Return content and start offset |
| `get_hash(path, lang, hash)` | Find an old record by hash |
| `get_hash_any(hash, lang)` | Find a record across paths, for renamed sources |
| `compact(paths)` | Keep only whitelisted paths and compact |
| `compact_with_first(paths, lang)` | Keep source language first and drop stale translations |

## Index and complexity

The first read scans the file once and builds `path -> lang -> offset` and
`hash\0lang\0path -> offset` indexes. Later reads seek directly, so a lookup
costs O(record length), with average O(1) index lookup. Compaction is a
streaming scan plus temporary-file replacement.

`get_hash_any` uses a `hash\0lang -> (path, offset)` index, so a renamed
source can reuse the translation cache from its old path; the caller appends
the result under the new path.
