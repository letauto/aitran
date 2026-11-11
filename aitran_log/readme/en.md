# aitran_log

Initialize rolling file logging for a project.

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
