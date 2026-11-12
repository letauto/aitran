[English](#en) | [中文](#zh)

---

<a name="en"></a>

# aitran

Batch-translate Markdown directories with caching, incremental revision, user-edit write-back and progress estimation.

- [Install](#install)
- [Config](#config)
- [Run](#run)
- [Cache](#cache)
- [Progress](#progress)
- [Features](#features)

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


---

<a name="zh"></a>

# aitran

按 Markdown 目录批量翻译，支持缓存、增量修订、用户译文回写和进度估算。

- [安装](#安装)
- [配置](#配置)
- [运行](#运行)
- [缓存](#缓存)
- [进度](#进度)
- [特性](#特性)

## 安装

```bash
cargo install aitran
```

## 配置

任务配置默认向上查找 `aitran.nt`：

```nt
-
  dir: ./i18n
  from: zh
  to:
    - en
    - ja
```

`from` 与 `to` 均为语言代码，取值见 [aitran_lang](https://crates.io/crates/aitran_lang) 语言表（如 `en`、`ja`、`zh-TW`）。

`to` 省略（或不填）时翻译为默认语言列表，即该表中的全部语言代码，并自动排除 `from`。

需要在 `~/.config/aii18n.nt` 配置 OpenAI 兼容的令牌，比如：

```nt
api:
  -
    base: https://api.example.com/v1
    token:
      - sk-xxx
    model: deepseek-v4.1-flash
    think: medium
```

## 运行

```bash
aitran
```

扫描 `dir/<from>/**.md`，输出到 `dir/<target>/<相对路径>`。

## 缓存

缓存在 `dir/.aitran/<from>.db`，记录头为：

```text
@path/lang/<源文件 blake3 base64url hash>
```

- 原文正文与译文分开保存
- 空行、纯符号行和前缀不进入缓存
- 译文逐条 trim
- 源文件 hash 用于找回旧原文
- 修改目标译文后再次运行，修改会追加回缓存，不会被模型覆盖
- 源文件重命名时按 hash 跨路径复用旧译文
- 源文件删除后清理目标文件，压实清理旧路径缓存

## 进度

每个实际翻译任务开始前显示整体进度和估算剩余时间。全部命中缓存的任务不计时、不占进度基数。

```text
[66.6%] 2/3
估算剩余时间 11s
开始翻译 ./i18n/zh/test.md → ja
```

## 特性

- 多 API 端点、多 token 轮转
- 流式思考与正文输出
- 按字符预算分批，多轮修订缺失/错误行
- 文件锁防止并发写数据库
- 只追加数据库，支持流式压实

