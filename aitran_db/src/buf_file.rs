//! 带位置的写缓冲，减少小写入系统调用

use std::{io, mem};

use compio::{buf::BufResult, fs::File, io::AsyncWriteAtExt};

use crate::{BUF_CAP, FIELD_SEP, HEADER_PREFIX, LINE_PREFIX};

/// 写缓冲：按记录头 + 内容行的格式追加，必要时整块落盘
pub(crate) struct BufFile<'a> {
  file: &'a File,
  pos: u64,
  buf: Vec<u8>,
}

impl<'a> BufFile<'a> {
  pub(crate) fn new(file: &'a File, pos: u64, cap: usize) -> Self {
    Self {
      file,
      pos,
      buf: Vec::with_capacity(cap),
    }
  }

  /// 已写入的末尾偏移（含缓冲区）
  pub(crate) fn written(&self) -> u64 {
    self.pos + self.buf.len() as u64
  }

  async fn push(&mut self, bytes: &[u8]) -> io::Result<()> {
    if bytes.len() > BUF_CAP.saturating_sub(self.buf.len()) {
      self.flush().await?;
      if bytes.len() < BUF_CAP {
        self.buf.extend_from_slice(bytes);
        return Ok(());
      }
      // 超长行按块直写，避免一次性复制整行
      for chunk in bytes.chunks(BUF_CAP) {
        let len = chunk.len() as u64;
        let BufResult(res, _) = self.file.write_all_at(chunk.to_vec(), self.pos).await;
        res?;
        self.pos += len;
      }
      return Ok(());
    }
    self.buf.extend_from_slice(bytes);
    Ok(())
  }

  /// 连续写入多段，整体能放进缓冲时一次扩展，否则逐段落盘
  async fn push_parts(&mut self, parts: &[&[u8]]) -> io::Result<()> {
    let need = parts
      .iter()
      .fold(0usize, |n, part| n.saturating_add(part.len()));
    if need <= BUF_CAP.saturating_sub(self.buf.len()) {
      self.buf.reserve(need);
      for part in parts {
        self.buf.extend_from_slice(part);
      }
      return Ok(());
    }
    for part in parts {
      self.push(part).await?;
    }
    Ok(())
  }

  /// 写入记录头（自动补换行）
  pub(crate) async fn push_head(&mut self, path: &str, lang: &str, hash: &str) -> io::Result<()> {
    self
      .push_parts(&[
        &[HEADER_PREFIX],
        path.as_bytes(),
        &[FIELD_SEP],
        lang.as_bytes(),
        &[FIELD_SEP],
        hash.as_bytes(),
        b"\n",
      ])
      .await
  }

  /// 写入内容行（自动补前缀与换行），内容不得含换行
  pub(crate) async fn push_line(&mut self, line: &[u8]) -> io::Result<()> {
    self.push_parts(&[&[LINE_PREFIX], line, b"\n"]).await
  }

  pub(crate) async fn flush(&mut self) -> io::Result<()> {
    if self.buf.is_empty() {
      return Ok(());
    }
    let len = self.buf.len() as u64;
    let buf = mem::take(&mut self.buf);
    let BufResult(res, mut buf) = self.file.write_all_at(buf, self.pos).await;
    buf.clear();
    self.buf = buf;
    self.pos += len;
    res
  }
}

/// 写一条完整记录，返回结束偏移
pub(crate) async fn record<S: AsRef<str>>(
  file: &File,
  pos: u64,
  path: &str,
  lang: &str,
  hash: &str,
  line_li: &[S],
  cap: usize,
) -> io::Result<u64> {
  let mut out = BufFile::new(file, pos, cap);
  out.push_head(path, lang, hash).await?;
  for line in line_li {
    out.push_line(line.as_ref().as_bytes()).await?;
  }
  out.flush().await?;
  Ok(out.written())
}
