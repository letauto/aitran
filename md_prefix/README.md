[English](#en) | [中文](#zh)

---

<a name="en"></a>

# md_prefix

Split and restore leading Markdown prefixes so translation only touches body text.

- [Install](#install)
- [Usage](#usage)
- [Prefix rules](#prefix-rules)
- [API](#api)

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


---

<a name="zh"></a>

# md_prefix

拆分并还原 Markdown 行首前缀。翻译时只处理正文，写回时保留原始缩进与结构标记。

- [安装](#安装)
- [用法](#用法)
- [前缀规则](#前缀规则)
- [接口](#接口)

## 安装

```toml
[dependencies]
md_prefix = "0.1"
```

## 用法

```rust
use md_prefix::{Part, restore};

let part = Part::new("  ## Title", true);
assert_eq!(part.prefix, "  ## ");
assert_eq!(part.body, "Title");
assert_eq!(restore(part.prefix, part.body), "  ## Title");
```

## 前缀规则

支持：

- 行首空白
- ATX 标题 `#` 到 `######`
- 引用 `>`
- 无序列表 `-`、`+`、`*`
- 有序列表 `1.`、`1)`
- 任务列表 `- [ ]`、`- [x]`

标记后必须是空白或行尾，因此 `#tag`、`-1`、`1.0`、`*强调*` 不会被误拆。

围栏代码块内可传 `markdown = false`，只拆空白缩进。

## 接口

| 接口 | 说明 |
| ---- | ---- |
| `Part::new(line, markdown)` | 拆分前缀与正文 |
| `restore(prefix, body)` | 重新拼接 |

