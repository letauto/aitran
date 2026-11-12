# aitran_parse

Line splitting, filtering and restoration for translated text, independent from model calls.

## Install

```toml
[dependencies]
aitran_parse = "0.1"
```

## Pipeline

For raw Markdown lines:

1. Split leading prefix from body text
2. Pass through empty lines and punctuation/emoji-only lines outside code fences
3. Send only translatable bodies to the model
4. Restore prefixes and pass-through lines by original line number

Punctuation-only lines inside fenced code blocks are still translated, constrained by the prompt to comments only.

## Usage

```rust
use aitran_parse::Txt;

let src_li = ["  ## Title", "```js", "});", "```", "..."];
let txt = Txt::new(&src_li);

assert_eq!(txt.line_li[0].prefix, "  ## ");
assert_eq!(txt.line_li[0].body, "Title");
assert_eq!(
  txt.restore(["Title", "```js", "});", "```"]),
  ["  ## Title", "```js", "});", "```", "..."]
);
```

## API

| Item | Description |
| ---- | ----------- |
| `Txt::new(&[&str])` | Split and classify in one pass |
| `Txt::line_li` | Translatable lines in source order |
| `Txt::body_li()` | Trimmed cache bodies, no empty lines |
| `Txt::restore(translations)` | Rebuild the full document, trimming each translation |
| `Line { nth, prefix, body }` | 1-based line number, prefix and body |
