[English](#en) | [中文](#zh)

---

<a name="en"></a>

# aitran_log

Initialize rolling file logging for a project.

- [Install](#install)
- [Usage](#usage)
- [Behavior](#behavior)
- [API](#api)

## Install

```toml
[dependencies]
aitran_log = "0.1"
```

## Usage

```rust
fn main() {
  if let Err(e) = aitran_log::init("aitran") {
    eprintln!("log init failed: {e}");
  }
  log::info!("hello");
}
```

## Behavior

- Level defaults to `info`; override with `RUST_LOG`
- Daily rollover, 16 MiB per file, 7 files kept
- Log directory prefers `dirs::state_dir()`, then `dirs::data_local_dir()`
  - Linux: `~/.local/state/<project>/`
  - macOS: `~/Library/Application Support/<project>/`
  - Windows: `%LOCALAPPDATA%\<project>\`
- File name is `<project>.log`

## API

| Item | Description |
| ---- | ----------- |
| `init(project)` | Initialize global logging once per process |
| `dir(project)` | Return the log directory |


---

<a name="zh"></a>

# aitran_log

按项目名初始化滚动文件日志。

- [安装](#安装)
- [用法](#用法)
- [行为](#行为)
- [接口](#接口)

## 安装

```toml
[dependencies]
aitran_log = "0.1"
```

## 用法

```rust
fn main() {
  if let Err(e) = aitran_log::init("aitran") {
    eprintln!("日志初始化失败: {e}");
  }
  log::info!("hello");
}
```

## 行为

- 默认级别 `info`，可用 `RUST_LOG` 覆盖
- 按天滚动，单文件上限 16 MiB，保留 7 个文件
- 日志目录优先 `dirs::state_dir()`，其次 `dirs::data_local_dir()`
  - Linux: `~/.local/state/<project>/`
  - macOS: `~/Library/Application Support/<project>/`
  - Windows: `%LOCALAPPDATA%\<project>\`
- 文件名为 `<project>.log`

## 接口

| 接口 | 说明 |
| ---- | ---- |
| `init(project)` | 初始化全局日志，同一进程只能调用一次 |
| `dir(project)` | 返回日志目录 |

