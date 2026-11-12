#![cfg_attr(docsrs, feature(doc_cfg))]

//! 按项目名初始化滚动文件日志。
//!
//! 默认级别 info，可用 `RUST_LOG` 覆盖。
//!
//! 日志目录优先 `dirs::state_dir()`（Linux `~/.local/state`），
//! 其次 `dirs::data_local_dir()`（macOS `~/Library/Application Support`，
//! Windows `%LOCALAPPDATA%`）。

mod error;

use std::{num::NonZeroUsize, path::PathBuf};

pub use error::{Error, Result};
use logforth::{
  append::file::FileBuilder, filter::rustlog::RustLogFilterBuilder, layout::TextLayout, starter_log,
};

/// 单个日志文件上限
const MAX_SIZE: NonZeroUsize = NonZeroUsize::new(16 * 1024 * 1024).unwrap();
/// 保留日志文件数
const MAX_FILES: NonZeroUsize = NonZeroUsize::new(7).unwrap();

/// 初始化文件日志，同一进程只能调用一次
pub fn init(project: &str) -> Result<()> {
  let file = FileBuilder::new(dir(project), project)
    .layout(TextLayout::default().no_color())
    .filename_suffix("log")
    .rollover_daily()
    .rollover_size(MAX_SIZE)
    .max_log_files(MAX_FILES)
    .build()?;
  starter_log::builder()
    .dispatch(move |d| {
      d.filter(RustLogFilterBuilder::from_default_env_or("info").build())
        .append(file)
    })
    .try_apply()?;
  Ok(())
}

/// 日志目录 `<state>/<project>`，日志文件为 `<project>.log`
fn dir(project: &str) -> PathBuf {
  dirs::state_dir()
    .or_else(dirs::data_local_dir)
    .or_else(|| dirs::home_dir().map(|home| home.join(".local/state")))
    .unwrap_or_else(|| PathBuf::from("."))
    .join(project)
}
