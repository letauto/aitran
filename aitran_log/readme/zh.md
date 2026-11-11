# aitran_log

按项目名初始化滚动文件日志。

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
