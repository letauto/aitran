# aitran

Batch-translate Markdown directories with caching, incremental revision, user-edit write-back and progress estimation.

## Install

```bash
cargo install aitran
```

## Config

The task config defaults to `aitran.nt`, searched upward:

```nt
-
  dir: ./i18n
  from: zh
  to:
    - en
    - ja
```

`from` and `to` are language codes; see the language table in [aitran_lang](https://crates.io/crates/aitran_lang) (e.g. `en`, `ja`, `zh-TW`).

When `to` is omitted (or left empty), every language in that table is translated, excluding `from`.

Put an OpenAI-compatible token in `~/.config/aii18n.nt`, for example:

```nt
api:
  -
    base: https://api.example.com/v1
    token:
      - sk-xxx
    model: deepseek-v4.1-flash
    think: medium
```

## Run

```bash
aitran
```

It scans `dir/<from>/**.md` and writes to `dir/<target>/<relative path>`.

## Cache

The cache lives at `dir/.aitran/<from>.db`. Record header:

```text
@path/lang/<source blake3 base64url hash>
```

- Source bodies and translations are stored separately
- Empty lines, punctuation-only lines and prefixes are not cached
- Every translation is trimmed
- The source hash locates the previous original
- Edits to target files are appended back and survive the next run
- Renamed source files reuse translations by hash across paths
- Deleted sources remove target files; compaction drops old path records

## Progress

Before each real translation job, overall progress and ETA are printed. Fully cached jobs are not timed and do not add to the progress base.

```text
[66.6%] 2/3
estimated 11s remaining
translating ./i18n/zh/test.md → ja
```

## Features

- Multiple API endpoints and token rotation
- Streaming reasoning and reply output
- Character-budget batching with multi-round repair
- File locking for safe concurrent cache access
- Append-only database with streaming compaction
