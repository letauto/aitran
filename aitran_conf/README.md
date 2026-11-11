[English](#en) | [中文](#zh)

---

<a name="en"></a>

# aitran_conf

NestedText task configuration loader for `aii18n.nt`.

- [Install](#install)
- [Format](#format)
- [Usage](#usage)
- [API](#api)

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


---

<a name="zh"></a>

# aitran_conf

解析 `aii18n.nt` 任务配置，使用 NestedText。

- [安装](#安装)
- [配置格式](#配置格式)
- [用法](#用法)
- [接口](#接口)

## 安装

```toml
[dependencies]
aitran_conf = "0.1"
```

## 配置格式

```text
-
  dir: ./i18n
  from: zh
  to:
    - en
```

| 字段 | 说明 |
| ---- | ---- |
| `dir` | 待翻译目录，必填 |
| `from` | 源语言，必填 |
| `to` | 目标语言；为空时展开为除源语言外的全部语言 |

语言代码由 `aitran_lang::by_code` 解析，大小写不敏感，`-` 与 `_` 等价。

## 用法

```rust
use aitran_conf::from_str;

let conf = from_str(
  "-\n  dir: ./i18n\n  from: zh\n  to:\n    - en\n",
)?.unwrap();

assert_eq!(conf.task[0].from_name(), "简体中文");
assert_eq!(conf.task[0].to_name().collect::<Vec<_>>(), ["English"]);
```

## 接口

| 接口 | 说明 |
| ---- | ---- |
| `from_str(text)` | 解析任务配置，空文档返回 `None` |
| `load_from(path)` | 读取文件并解析，文件不存在返回 `None` |
| `parse_as::<T>(text)` | 解析任意 NestedText 结构 |
| `load_as::<T>(path)` | 读取并解析任意 NestedText 结构 |

