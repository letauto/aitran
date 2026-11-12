[English](#en) | [中文](#zh)

---

<a name="en"></a>

# aitran_cli_conf

Config loader for `ai_i18n`. Reads a [NestedText](https://nestedtext.org) file and deserializes it into `Conf` via serde.

- [Install](#install)
- [Config path](#config-path)
- [Config file](#config-file)
- [Usage](#usage)
- [API](#api)
- [Errors](#errors)

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


---

<a name="zh"></a>

# aitran_cli_conf

`ai_i18n` 的配置加载器。读取 [NestedText](https://nestedtext.org) 文件，通过 serde 反序列化为 `Conf`。

- [安装](#安装)
- [配置路径](#配置路径)
- [配置文件](#配置文件)
- [使用](#使用)
- [接口](#接口)
- [错误](#错误)

## 安装

```toml
[dependencies]
aitran_cli_conf = "0.1"
```

## 配置路径

1. 环境变量 `AII18N_CONF_PATH`：直接指定配置文件完整路径
2. 未设置时回退到 `<系统配置目录>/aii18n.nt`（如 `~/.config/aii18n.nt`）

## 配置文件

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

`api` 的每一项对应 `Api`（结构体定义在 `ai_api`）：

| 字段    | 类型          | 说明             |
| ------- | ------------- | ---------------- |
| `base`  | `String`      | 接口地址         |
| `token` | `Vec<String>` | 可用 token，按顺序尝试 |
| `model` | `String`      | 模型名           |
| `think` | `Think`       | 思考强度，缺省 `medium` |

注意：NestedText 不支持列表项内联映射，`- base: ...` 是**非法**写法。`-` 必须独占一行，字典字段缩进在其下方。

## 使用

```rust
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

解析字符串或指定文件：

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

也可以用自己的结构体：`aitran_conf::parse_as` / `load_as` 对任意 `Deserialize` 类型泛型。

## 接口

| 函数 | 说明 |
| ---- | ---- |
| `conf() -> &'static Path` | 解析后的配置文件路径 |
| `load() -> Result<Option<Conf>>` | 解析当前配置文件 |

解析文本、指定文件与自定义结构体见 `aitran_conf`（`parse_as` / `load_as`）。

## 错误

错误类型是 `aitran_conf::Error`，共三个变体：

- `Error::Io`：读取文件失败（`NotFound` 除外，此时返回 `Ok(None)`）
- `Error::Parse`：NestedText 语法或类型错误，带行列信息
- `Error::Lang`：语言代码未知（自定义结构体反序列化时不会出现）

