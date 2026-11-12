//! 键索引与记录体读取

use std::io;

use compio::io::AsyncReadAt;
use gxhash::{HashMap, HashSet};

use crate::{FIELD_SEP, HEADER_PREFIX, LINE_PREFIX, line_reader::LineReader, norm_path};

/// lang -> 最后一条记录的起始偏移
pub(crate) type LangMap = HashMap<Box<str>, u64>;

/// path -> lang -> 最后一条记录的起始偏移
pub(crate) type Latest = HashMap<Box<str>, LangMap>;

/// `hash\0lang\0path` -> 最后一条记录的起始偏移
pub(crate) type HashLatest = HashMap<Box<str>, u64>;

/// `hash\0lang` -> (path, 最后一条记录的起始偏移)
pub(crate) type HashAny = HashMap<Box<str>, (Box<str>, u64)>;

/// 三套键索引的聚合，写入与扫描后统一维护
#[derive(Default)]
pub(crate) struct Ix {
  /// 每个 `@path/lang` 最后一条记录的起始偏移
  pub(crate) latest: Latest,
  /// 每个 `(hash, lang, path)` 最后一条记录的起始偏移
  pub(crate) hash: HashLatest,
  /// 每个 `(hash, lang)` 最后一条记录的位置
  pub(crate) hash_any: HashAny,
}

impl Ix {
  /// 更新全部索引：同键只保留最后一条
  pub(crate) fn update(&mut self, head: &Head<'_>, off: u64) {
    update_latest(&mut self.latest, head.path, head.lang, off);
    update_hash(&mut self.hash, head, off);
    update_hash_any(&mut self.hash_any, head, off);
  }
}

/// 记录头视图：`@path/lang/hash`
#[derive(Clone, Copy)]
pub(crate) struct Head<'a> {
  pub(crate) path: &'a str,
  pub(crate) lang: &'a str,
  pub(crate) hash: &'a str,
}

impl<'a> Head<'a> {
  /// 解析一行；非记录头返回 `None`
  pub(crate) fn parse(line: &'a [u8]) -> Option<Self> {
    // 内容行占多数，先按首字节排除，避免每行都做 UTF-8 校验
    if line.first() != Some(&HEADER_PREFIX) {
      return None;
    }
    let raw = str::from_utf8(trim_cr(line)).ok()?;
    let (path_lang, hash) = raw[1..].rsplit_once(FIELD_SEP as char)?;
    let (path, lang) = path_lang.rsplit_once(FIELD_SEP as char)?;
    (!path.is_empty() && !lang.is_empty() && !hash.is_empty()).then_some(Self { path, lang, hash })
  }
}

/// 记录体：从记录头之后逐行读取，遇到下一条记录头即结束
pub(crate) struct Body<'r, F> {
  reader: &'r mut LineReader<F>,
  pub(crate) hash: String,
}

impl<'r, F: AsyncReadAt> Body<'r, F> {
  /// 读取记录头，校验 `path` / `lang` 后返回记录体
  pub(crate) async fn open(
    reader: &'r mut LineReader<F>,
    path: &str,
    lang: &str,
  ) -> io::Result<Option<Self>> {
    let Some((line, _)) = reader.next().await? else {
      return Ok(None);
    };
    let hash = match Head::parse(line) {
      Some(head) if head.path == path && head.lang == lang => head.hash.to_owned(),
      _ => return Ok(None),
    };
    Ok(Some(Self { reader, hash }))
  }

  /// 下一条内容行（不含前缀与行尾 `\r`）；记录结束返回 `None`
  pub(crate) async fn next(&mut self) -> io::Result<Option<&[u8]>> {
    loop {
      let Some((line, _)) = self.reader.next().await? else {
        return Ok(None);
      };
      match line.first() {
        Some(&HEADER_PREFIX) => return Ok(None),
        Some(&LINE_PREFIX) => return Ok(Some(trim_cr(&line[1..]))),
        _ => {}
      }
    }
  }
}

/// 更新索引：键已存在只改偏移，不存在才分配键
pub(crate) fn update_latest(map: &mut Latest, path: &str, lang: &str, off: u64) {
  match map.get_mut(path) {
    Some(lang_map) => match lang_map.get_mut(lang) {
      Some(slot) => *slot = off,
      None => {
        lang_map.insert(lang.into(), off);
      }
    },
    None => {
      let mut lang_map = LangMap::default();
      lang_map.insert(lang.into(), off);
      map.insert(path.into(), lang_map);
    }
  }
}

/// 更新 hash 索引：同键只保留最后一条
fn update_hash(map: &mut HashLatest, head: &Head<'_>, off: u64) {
  map.insert(hash_key(head.path, head.lang, head.hash).into(), off);
}

/// 更新跨路径 hash 索引：同 hash/lang 只保留最后一条
fn update_hash_any(map: &mut HashAny, head: &Head<'_>, off: u64) {
  map.insert(
    hash_lang_key(head.hash, head.lang).into(),
    (head.path.into(), off),
  );
}

/// 构造 hash 索引键；三段均不含 NUL
pub(crate) fn hash_key(path: &str, lang: &str, hash: &str) -> String {
  let mut key = String::with_capacity(
    path
      .len()
      .saturating_add(lang.len())
      .saturating_add(hash.len())
      .saturating_add(2),
  );
  key.push_str(hash);
  key.push('\0');
  key.push_str(lang);
  key.push('\0');
  key.push_str(path);
  key
}

/// 构造跨路径 hash 索引键
pub(crate) fn hash_lang_key(hash: &str, lang: &str) -> String {
  let mut key = String::with_capacity(hash.len().saturating_add(lang.len()).saturating_add(1));
  key.push_str(hash);
  key.push('\0');
  key.push_str(lang);
  key
}

/// 扫描 `reader` 建立索引；`keep` 为 `Some` 时只保留白名单路径，
/// `with_hash` 为 `false` 时跳过 hash 索引（压实第一遍只需偏移）。
pub(crate) async fn scan_latest<F: AsyncReadAt>(
  reader: &mut LineReader<F>,
  keep: Option<&HashSet<Box<str>>>,
  with_hash: bool,
) -> io::Result<Ix> {
  let mut ix = Ix::default();
  // 白名单为空时无需扫描
  if keep.is_some_and(HashSet::is_empty) {
    return Ok(ix);
  }
  while let Some((line, off)) = reader.next().await? {
    let Some(head) = Head::parse(line) else {
      continue;
    };
    if let Some(keep) = keep
      && !keep.contains(head.path)
    {
      continue;
    }
    if with_hash {
      ix.update(&head, off);
    } else {
      update_latest(&mut ix.latest, head.path, head.lang, off);
    }
  }
  Ok(ix)
}

/// 归一化白名单路径
pub(crate) fn whitelist<S: AsRef<str>>(path_li: &[S]) -> HashSet<Box<str>> {
  path_li
    .iter()
    .map(|p| norm_path(p.as_ref()).into_owned().into_boxed_str())
    .collect()
}

/// 去掉行尾 `\r`（容忍 CRLF 手工编辑）
fn trim_cr(line: &[u8]) -> &[u8] {
  match line.last() {
    Some(b'\r') => &line[..line.len() - 1],
    _ => line,
  }
}
