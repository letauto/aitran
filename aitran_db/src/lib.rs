//! 纯文本、可 diff 的 i18n 数据库。
//!
//! 存储格式（只追加）：
//!
//! ```text
//! @path/lang/hash
//! >line
//! >line
//! ```
//!
//! - `@` 开头是记录头，按最后两个 `/` 拆出 path、lang 与 hash
//! - `hash` 为源文件内容的 BLAKE3 base64url，用于找回译文对应的旧原文
//! - `>` 开头是内容行
//! - 其他行忽略
//! - 同一 `@path/lang` 后写覆盖先写（读取时最后一条生效）
//!
//! 首次读取扫描一次文件建立 `path -> lang -> 最后偏移` 与 hash 索引，之后按偏移直读，
//! 单次读取只扫描目标记录；压实仍为流式扫描。索引内存 O(唯一键数)。
//! 进程内的写入与压实会同步维护索引；外部手工改文件后需重新 `open`。

mod buf_file;
mod db;
mod error;
mod index;
mod line_reader;

use std::borrow::Cow;

pub use db::{Db, Record};
pub use error::{Error, Result};

/// 记录头前缀
const HEADER_PREFIX: u8 = b'@';
/// 内容行前缀
const LINE_PREFIX: u8 = b'>';
/// 记录头中 path 与 lang 的分隔符
const FIELD_SEP: u8 = b'/';
/// 读写缓冲块大小。
pub(crate) const BUF_CAP: usize = 64 * 1024;

/// Windows 下把路径分隔符统一成 `/`，其余平台零拷贝。
#[cfg(windows)]
pub(crate) fn norm_path(path: &str) -> Cow<'_, str> {
  if path.as_bytes().contains(&b'\\') {
    Cow::Owned(path.replace('\\', FIELD_SEP as char))
  } else {
    Cow::Borrowed(path)
  }
}

#[cfg(not(windows))]
pub(crate) fn norm_path(path: &str) -> Cow<'_, str> {
  Cow::Borrowed(path)
}
