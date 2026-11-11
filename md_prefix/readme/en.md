# md_prefix

Split and restore leading Markdown prefixes so translation only touches body text.

## Install

```toml
[dependencies]
md_prefix = "0.1"
```

## Usage

```rust
use md_prefix::{Part, restore};

let part = Part::new("  ## Title", true);
assert_eq!(part.prefix, "  ## ");
assert_eq!(part.body, "Title");
assert_eq!(restore(part.prefix, part.body), "  ## Title");
```

## Prefix rules

Supported prefixes:

- Leading whitespace
- ATX headings `#` through `######`
- Block quotes `>`
- Bullet lists `-`, `+`, `*`
- Ordered lists `1.`, `1)`
- Task lists `- [ ]`, `- [x]`

A marker must be followed by whitespace or end of line, so `#tag`, `-1`, `1.0` and `*emphasis*` are not split.

Inside fenced code blocks, pass `markdown = false` to strip indentation only.

## API

| Item | Description |
| ---- | ----------- |
| `Part::new(line, markdown)` | Split prefix and body |
| `restore(prefix, body)` | Join them back |
