# aitran_cli_conf

Config loader for `ai_i18n`. Reads a [NestedText](https://nestedtext.org) file and deserializes it into `Conf` via serde.

## Install

```toml
[dependencies]
aitran_cli_conf = "0.1"
```

## Config path

1. `AII18N_CONF_PATH`: full path to the config file
2. Fallback: `<system config dir>/aii18n.nt` (e.g. `~/.config/aii18n.nt` on Linux/macOS)

## Config file

```nt
# ai_i18n API 配置
api:
  -
    base: https://api.example.com/v1
    token:
      - sk-xxx
      - sk-yyy
    model: deepseek-v4.1-flash
    think: medium
```

Each entry in `api` maps to `Api` (defined in `ai_api`):

| Field   | Type       | Description                          |
| ------- | ---------- | ------------------------------------ |
| `base`  | `String`   | API endpoint                         |
| `token` | `Vec<String>` | Tokens, tried in order            |
| `model` | `String`   | Model name                           |
| `think` | `Think`    | Thinking level, `medium` by default  |

Note: NestedText has no inline mapping for list items, so `- base: ...` is **invalid**. The `-` must be alone on its line, with the dict fields indented below.

## Usage

```rust,no_run
use aitran_cli_conf::Conf;
use aitran_conf::Error;

fn main() -> Result<(), Error> {
  if let Some(conf: Conf) = aitran_cli_conf::load()? {
    for api in conf.api_li {
      println!("{} {} {}", api.base, api.model, api.token_li.len());
    }
  }
  Ok(())
}
```

Parse a string or a specific file:

```rust
use std::path::Path;

use aitran_cli_conf::Conf;
use aitran_conf::Error;

fn demo() -> Result<(), Error> {
  let text = r#"api:
  -
    base: https://api.example.com/v1
    token:
      - sk-xxx
    model: deepseek-v4.1-flash
"#;
  let from_text: Option<Conf> = aitran_conf::parse_as(text)?;
  let from_file: Option<Conf> = aitran_conf::load_as(Path::new("/path/to/aii18n.nt"))?;
  println!("{from_text:?} {from_file:?}");
  Ok(())
}
```

Any `Deserialize` type works with `aitran_conf::parse_as` / `load_as`, so you can use your own structs instead of `Conf`.

## API

| Function | Description |
| -------- | ----------- |
| `conf() -> &'static Path` | Resolved config file path |
| `load() -> Result<Option<Conf>>` | Parse the resolved config file |

Parsing text, arbitrary files and custom structs lives in `aitran_conf` (`parse_as` / `load_as`).

## Errors

The error type is `aitran_conf::Error`, with three variants:

- `Error::Io`: reading the file failed (except `NotFound`, which yields `Ok(None)`)
- `Error::Parse`: NestedText syntax or type error, with line and column info
- `Error::Lang`: unknown language code (never hit when deserializing custom structs)
