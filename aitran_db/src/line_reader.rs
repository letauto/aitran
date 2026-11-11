use std::{io, mem};

use compio::{buf::BufResult, io::AsyncReadAt};
use memchr::memchr;

use crate::BUF_CAP;

/// 流式按行读取器：常驻内存 O(`BUF_CAP`)，超长行额外占用该行长度。
///
/// 返回的行不含 `\n`，文件末尾没有 `\n` 的最后一行也会返回。
pub(crate) struct LineReader<F> {
  file: F,
  buf: Vec<u8>,
  spill: Vec<u8>,
  start: usize,
  base: u64,
  eof: bool,
}

impl<F: AsyncReadAt> LineReader<F> {
  pub(crate) fn new(file: F, pos: u64) -> Self {
    Self {
      file,
      buf: Vec::with_capacity(BUF_CAP),
      spill: Vec::new(),
      start: 0,
      base: pos,
      eof: false,
    }
  }

  /// 重新定位到 `pos`，复用已分配的缓冲区。
  pub(crate) fn seek_to(&mut self, pos: u64) {
    // 目标仍在当前缓冲内时直接移动游标，省一次系统调用
    if pos >= self.base
      && let Ok(rel) = usize::try_from(pos - self.base)
      && rel < self.buf.len()
    {
      self.start = rel;
      self.spill.clear();
      self.eof = false;
      return;
    }
    self.buf.clear();
    self.spill.clear();
    self.start = 0;
    self.base = pos;
    self.eof = false;
  }

  /// 填充缓冲区，返回是否读到新数据。
  ///
  /// 调用前缓冲区必然已全部消费（`start == buf.len()`）。
  async fn fill(&mut self) -> io::Result<bool> {
    if self.eof {
      return Ok(false);
    }
    debug_assert_eq!(self.start, self.buf.len());
    self.base += self.buf.len() as u64;
    self.buf.clear();
    self.start = 0;
    let buf = mem::take(&mut self.buf);
    let BufResult(res, buf) = self.file.read_at(buf, self.base).await;
    self.buf = buf;
    match res {
      Ok(0) => {
        self.eof = true;
        Ok(false)
      }
      Ok(_) => Ok(true),
      Err(e) => Err(e),
    }
  }

  /// 读取下一行及其文件起始偏移；EOF 返回 `None`。
  pub(crate) async fn next(&mut self) -> io::Result<Option<(&[u8], u64)>> {
    self.spill.clear();
    let line_off = self.base + self.start as u64;
    let mut spanning = false;
    loop {
      if self.start == self.buf.len() && !self.fill().await? {
        return Ok(if spanning {
          Some((&self.spill, line_off))
        } else {
          None
        });
      }
      let start = self.start;
      match memchr(b'\n', &self.buf[start..]) {
        Some(i) => {
          let end = start + i;
          self.start = end + 1;
          if spanning {
            self.spill.extend_from_slice(&self.buf[start..end]);
            return Ok(Some((&self.spill, line_off)));
          }
          return Ok(Some((&self.buf[start..end], line_off)));
        }
        None => {
          spanning = true;
          self.spill.extend_from_slice(&self.buf[start..]);
          self.start = self.buf.len();
        }
      }
    }
  }
}
