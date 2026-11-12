# aitran_api

Line-by-line batch translation: preprocessing, prompt generation, batching, reply parsing and multi-round repair.

## Install

```toml
[dependencies]
aitran_api = "0.1"
```

## Pipeline

1. `aitran_parse::Txt` splits prefixes and bodies, filtering empty and punctuation-only lines
2. Split into batches by character budget
3. Build `line: source` prompts
4. Parse `line / source / translation` blocks from the reply
5. Missing, wrong-source and empty-translation lines go to the next round
6. Stop only after 3 rounds without progress

## Usage

```rust
use aitran_api::tran_with;

let out = tran_with(
  &api_li,
  "zh",
  "en",
  &["你好", "世界"],
).await?;
assert_eq!(out, ["Hello", "World"]);
```

## API

| Item | Description |
| ---- | ----------- |
| `tran_with` | Translate without callbacks |
| `tran_ask_hook` | Streaming reasoning/content/prompt callbacks |
| `tran_ask_hook_cache` | Line cache plus prompt callback |
| `tran_with_limit::<LIMIT>` | Custom character limit, useful in tests |
| `CHAR_LIMIT` | Default 10,000 characters per batch |
| `HookSet` | Reasoning, reply and prompt callbacks |
| `OnPrompt` | Capture system and actual user prompts |

## Repair

- Replies are split by a line containing only `-`
- Each block contains line number, source and translation
- Source must match the input exactly
- Translations are trimmed to one line
- Pending lines only shrink; after `STALL = 3` rounds without progress, `Error::Format` is returned
