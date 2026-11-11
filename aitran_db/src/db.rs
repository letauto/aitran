//! 数据库主体：打开、追加、读取与压实

use std::{
  borrow::Cow,
  fs::{File as StdFile, OpenOptions as StdOpenOptions, TryLockError},
  io,
  path::{Path, PathBuf},
};

use async_lock::RwLock;
use compio::fs::{File, OpenOptions, rename};
use gxhash::{HashMap, HashMapExt};
use memchr::{memchr, memchr2, memchr3};

use crate::{
  Error, FIELD_SEP, Result,
  buf_file::{BufFile, record},
  index::{
    Body, HashAny, HashLatest, Head, LangMap, Latest, Scan, hash_key, hash_lang_key, scan_latest,
    update_hash, update_hash_any, update_latest, whitelist,
  },
  line_reader::LineReader,
  norm_path,
};

/// 只追加文本库。
///
/// `open` 会对 `<db>.lock` 加独占文件锁，禁止跨进程并发访问；
/// 进程内读写由异步读写锁协调。
pub struct Db {
  path: PathBuf,
  lock: RwLock<Writer>,
  _file_lock: StdFile,
}

/// 一条记录的内容与起始偏移。
pub struct Record {
  pub line_li: Vec<String>,
  pub off: u64,
  /// 记录头中的源文件 hash
  pub hash: String,
}

#[derive(Default)]
struct Writer {
  file: Option<File>,
  end: u64,
  /// 每个 `@path/lang` 最后一条记录的起始偏移；`None` 表示尚未建立。
  latest: Option<Latest>,
  /// 每个 `(path, lang, hash)` 最后一条记录的起始偏移。
  hash: Option<HashLatest>,
  /// 每个 `(hash, lang)` 最后一条记录的位置。
  hash_any: Option<HashAny>,
}

impl Db {
  /// 打开数据库并获取独占文件锁；被其他实例或进程占用时返回 [`Error::Locked`]。
  ///
  /// 文件不存在时首次写入自动创建。
  pub fn open(path: impl Into<PathBuf>) -> Result<Self> {
    let path = path.into();
    let lock_path = sibling(&path, ".lock");
    let file_lock = StdOpenOptions::new()
      .create(true)
      .truncate(false)
      .read(true)
      .write(true)
      .open(&lock_path)?;
    match file_lock.try_lock() {
      Ok(()) => {}
      Err(TryLockError::WouldBlock) => return Err(Error::Locked(lock_path)),
      Err(TryLockError::Error(e)) => return Err(e.into()),
    }
    Ok(Self {
      path,
      lock: RwLock::new(Writer::default()),
      _file_lock: file_lock,
    })
  }

  pub fn path(&self) -> &Path {
    &self.path
  }

  /// 追加一条记录：同一 `(path, lang)` 后写覆盖先写。
  ///
  /// `hash` 是源文件内容的 BLAKE3 base64url。
  pub async fn set<S: AsRef<str>>(
    &self,
    path: &str,
    lang: &str,
    hash: &str,
    line_li: &[S],
  ) -> Result<()> {
    let (path, lang) = key(path, lang)?;
    let hash = check_hash(hash)?;
    // 先校验全部行并算出记录长度，避免写出半条记录后再失败
    let mut cap = path
      .len()
      .saturating_add(lang.len())
      .saturating_add(hash.len())
      .saturating_add(4);
    for line in line_li {
      let bytes = line.as_ref().as_bytes();
      if memchr2(b'\n', b'\r', bytes).is_some() {
        return Err(Error::BadLine);
      }
      cap = cap.saturating_add(bytes.len().saturating_add(2));
    }

    let mut w = self.lock.write().await;
    let (file, pos) = self.ensure_writer(&mut w).await?;
    let end = match record(
      file,
      pos,
      &path,
      lang,
      hash,
      line_li,
      cap.min(crate::BUF_CAP),
    )
    .await
    {
      Ok(end) => end,
      Err(e) => {
        // 回滚半条记录，避免后续追加把残留内容留在文件里
        file.set_len(pos).await?;
        return Err(e.into());
      }
    };
    w.end = end;
    if let Some(latest) = &mut w.latest {
      update_latest(latest, &path, lang, pos);
    }
    if let Some(map) = &mut w.hash {
      update_hash(
        map,
        &Head {
          path: &path,
          lang,
          hash,
        },
        pos,
      );
    }
    if let Some(map) = &mut w.hash_any {
      update_hash_any(
        map,
        &Head {
          path: &path,
          lang,
          hash,
        },
        pos,
      );
    }
    Ok(())
  }

  /// 读取 `(path, lang)` 的最新内容；不存在返回空 `Vec`。
  pub async fn get(&self, path: &str, lang: &str) -> Result<Vec<String>> {
    Ok(
      self
        .get_with_off(path, lang)
        .await?
        .map_or_else(Vec::new, |record| record.line_li),
    )
  }

  /// 读取最新内容与记录起始偏移；不存在返回 `None`。
  pub async fn get_with_off(&self, path: &str, lang: &str) -> Result<Option<Record>> {
    let (path, lang) = key(path, lang)?;
    // 读锁防止读取期间发生追加或压实；索引缺失时再升级为写锁建立
    let mut guard = self.lock.read().await;
    if guard.latest.is_none() || guard.hash.is_none() {
      drop(guard);
      // 用块限定写锁，读完必须释放，否则再次获取读锁会自锁
      {
        let mut w = self.lock.write().await;
        if w.latest.is_none() || w.hash.is_none() {
          self.build_latest(&mut w).await?;
        }
      }
      guard = self.lock.read().await;
    }
    let Some(off) = guard
      .latest
      .as_ref()
      .and_then(|m| m.get(path.as_ref()))
      .and_then(|m| m.get(lang))
      .copied()
    else {
      return Ok(None);
    };
    self.read(&guard, path.as_ref(), lang, off, None).await
  }

  /// 按源文件 hash 读取记录，用于找回译文对应的旧原文。
  pub async fn get_hash(&self, path: &str, lang: &str, hash: &str) -> Result<Option<Record>> {
    let (path, lang) = key(path, lang)?;
    let hash = check_hash(hash)?;
    let mut guard = self.lock.read().await;
    if guard.latest.is_none() || guard.hash.is_none() {
      drop(guard);
      {
        let mut w = self.lock.write().await;
        if w.latest.is_none() || w.hash.is_none() {
          self.build_latest(&mut w).await?;
        }
      }
      guard = self.lock.read().await;
    }
    let Some(off) = guard
      .hash
      .as_ref()
      .and_then(|m| m.get(&hash_key(path.as_ref(), lang, hash)))
      .copied()
    else {
      return Ok(None);
    };
    self
      .read(&guard, path.as_ref(), lang, off, Some(hash))
      .await
  }

  /// 跨路径按 hash 读取记录，用于识别源文件重命名并复用译文。
  pub async fn get_hash_any(&self, hash: &str, lang: &str) -> Result<Option<Record>> {
    let hash = check_hash(hash)?;
    let lang = check_lang(lang)?;
    let mut guard = self.lock.read().await;
    if guard.latest.is_none() || guard.hash_any.is_none() {
      drop(guard);
      {
        let mut w = self.lock.write().await;
        if w.latest.is_none() || w.hash_any.is_none() {
          self.build_latest(&mut w).await?;
        }
      }
      guard = self.lock.read().await;
    }
    let Some((path, off)) = guard
      .hash_any
      .as_ref()
      .and_then(|m| m.get(&hash_lang_key(hash, lang)))
    else {
      return Ok(None);
    };
    self.read(&guard, path, lang, *off, Some(hash)).await
  }

  /// 从已定位的偏移读取记录
  async fn read(
    &self,
    guard: &Writer,
    path: &str,
    lang: &str,
    off: u64,
    expect_hash: Option<&str>,
  ) -> Result<Option<Record>> {
    // 写句柄已打开时直接复用；否则按需打开
    let opened;
    let file = match &guard.file {
      Some(file) => file,
      None => {
        opened = match open_read(&self.path).await? {
          Some(file) => file,
          None => return Ok(None),
        };
        &opened
      }
    };
    // 索引指向最后一条同键记录，遇到下一个记录头即结束
    let mut reader = LineReader::new(file, off);
    let Some(mut body) = Body::open(&mut reader, path, lang).await? else {
      return Ok(None);
    };
    if expect_hash.is_some_and(|hash| body.hash != hash) {
      return Ok(None);
    }
    let mut out = Vec::new();
    while let Some(line) = body.next().await? {
      out.push(str::from_utf8(line)?.to_owned());
    }
    Ok(Some(Record {
      line_li: out,
      off,
      hash: body.hash,
    }))
  }

  /// 压实：只保留 `path_li` 中的路径，其余路径全部删除。
  ///
  /// - `path_li` 是白名单，白名单外的记录会被丢弃
  /// - `path_li` 为空：清空数据库
  /// - 压实部分按 `(path, lang)` 排序，只保留每个键的最后一条
  /// - 流式扫描 + 临时文件 + `mv` 覆盖
  pub async fn compact<S: AsRef<str>>(&self, path_li: &[S]) -> Result<()> {
    self.compact_with_first(path_li, None).await
  }

  /// 压实，并让 `first` 指定的语言记录排在同路径其他语言之前。
  ///
  /// 译文有效性依赖记录先后：早于最新原文记录的译文已经失效，直接丢弃。
  pub async fn compact_with_first<S: AsRef<str>>(
    &self,
    path_li: &[S],
    first: Option<&str>,
  ) -> Result<()> {
    let keep = whitelist(path_li);
    let mut w = self.lock.write().await;
    // 先关旧句柄，Windows 下才能 rename 覆盖
    w.file = None;
    let Some(read) = open_read(&self.path).await? else {
      w.end = 0;
      w.latest = Some(Latest::default());
      w.hash = Some(HashLatest::default());
      w.hash_any = Some(HashAny::default());
      return Ok(());
    };

    // 第一遍：记录白名单内每个键最后一条的偏移，内存 O(键数)
    let mut reader = LineReader::new(&read, 0);
    let Scan { latest, .. } = scan_latest(&mut reader, Some(&keep), false).await?;
    drop(keep);
    let mut paths: Vec<(Box<str>, LangMap)> = latest.into_iter().collect();
    paths.sort_unstable_by(|a, b| a.0.cmp(&b.0));

    let tmp = self.tmp_path();
    let tmp_file = OpenOptions::new()
      .write(true)
      .create(true)
      .truncate(true)
      .open(&tmp)
      .await?;
    let mut new_latest = Latest::with_capacity(paths.len());
    let mut new_hash = HashLatest::default();
    let mut new_hash_any = HashAny::default();
    let end = {
      let mut out = BufFile::new(&tmp_file, 0, crate::BUF_CAP);
      // 第二遍：按序重写白名单记录，内容流式搬运
      for (path, lang_map) in paths {
        let src_off = first.and_then(|first| lang_map.get(first)).copied();
        let mut langs: Vec<(Box<str>, u64)> = lang_map.into_iter().collect();
        langs.sort_unstable_by(|a, b| {
          let af = first.is_some_and(|first| a.0.as_ref() == first);
          let bf = first.is_some_and(|first| b.0.as_ref() == first);
          bf.cmp(&af).then_with(|| a.0.cmp(&b.0))
        });
        let mut new_langs = HashMap::with_capacity(langs.len());
        for (lang, off) in langs {
          // 原文变更后，旧译文仍留在原文记录之前，压实后不能再排到原文之后
          if first.is_some_and(|first| first != lang.as_ref())
            && src_off.is_some_and(|src| off < src)
          {
            continue;
          }
          reader.seek_to(off);
          let Some(mut body) = Body::open(&mut reader, &path, &lang).await? else {
            return Err(io::Error::other("记录头与索引不一致").into());
          };
          let new_off = out.written();
          out.push_head(&path, &lang, &body.hash).await?;
          update_hash(
            &mut new_hash,
            &Head {
              path: &path,
              lang: &lang,
              hash: &body.hash,
            },
            new_off,
          );
          update_hash_any(
            &mut new_hash_any,
            &Head {
              path: &path,
              lang: &lang,
              hash: &body.hash,
            },
            new_off,
          );
          while let Some(line) = body.next().await? {
            out.push_line(line).await?;
          }
          new_langs.insert(lang, new_off);
        }
        new_latest.insert(path, new_langs);
      }
      out.flush().await?;
      out.written()
    };
    tmp_file.sync_all().await?;
    drop(tmp_file);
    // 复用过的 reader 也持有读句柄，Windows 下必须关闭才能 rename
    drop(reader);
    drop(read);
    rename(&tmp, &self.path).await?;
    // 追加位置必须跟随新文件长度，否则会覆盖压实内容
    w.end = end;
    w.latest = Some(new_latest);
    w.hash = Some(new_hash);
    w.hash_any = Some(new_hash_any);
    Ok(())
  }

  /// 扫描整个文件，建立每个记录头最后一条的偏移索引。
  async fn build_latest(&self, w: &mut Writer) -> Result<()> {
    let scan = match open_read(&self.path).await? {
      Some(file) => {
        let mut reader = LineReader::new(&file, 0);
        scan_latest(&mut reader, None, true).await?
      }
      None => Scan {
        latest: Latest::default(),
        hash: HashLatest::default(),
        hash_any: HashAny::default(),
      },
    };
    w.latest = Some(scan.latest);
    w.hash = Some(scan.hash);
    w.hash_any = Some(scan.hash_any);
    Ok(())
  }

  async fn ensure_writer<'a>(&self, w: &'a mut Writer) -> Result<(&'a mut File, u64)> {
    if w.file.is_none() {
      let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&self.path)
        .await?;
      w.end = file.metadata().await?.len();
      w.file = Some(file);
    }
    let pos = w.end;
    let file = w
      .file
      .as_mut()
      .ok_or_else(|| io::Error::other("writer 未初始化"))?;
    Ok((file, pos))
  }

  fn tmp_path(&self) -> PathBuf {
    sibling(&self.path, ".tmp")
  }
}

/// 在原路径后追加后缀，生成同目录下的兄弟路径
fn sibling(path: &Path, suffix: &str) -> PathBuf {
  let mut name = path.as_os_str().to_os_string();
  name.push(suffix);
  PathBuf::from(name)
}

/// 打开只读句柄；文件不存在返回 `None`。
async fn open_read(path: &Path) -> Result<Option<File>> {
  match File::open(path).await {
    Ok(file) => Ok(Some(file)),
    Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
    Err(e) => Err(e.into()),
  }
}

/// 校验并规范化 `(path, lang)`。
fn key<'a>(path: &'a str, lang: &'a str) -> Result<(Cow<'a, str>, &'a str)> {
  let path = norm_path(path);
  if path.is_empty() {
    return Err(Error::EmptyPath);
  }
  if memchr2(b'\n', b'\r', path.as_bytes()).is_some() || memchr(0, path.as_bytes()).is_some() {
    return Err(Error::BadPath(path.into_owned()));
  }
  let lang = check_lang(lang)?;
  Ok((path, lang))
}

/// 校验语言代码
fn check_lang(lang: &str) -> Result<&str> {
  if lang.is_empty() {
    return Err(Error::EmptyLang);
  }
  if memchr3(FIELD_SEP, b'\n', b'\r', lang.as_bytes()).is_some()
    || memchr(0, lang.as_bytes()).is_some()
  {
    return Err(Error::BadLang(lang.to_owned()));
  }
  Ok(lang)
}

/// 校验记录头中的 hash 段
fn check_hash(hash: &str) -> Result<&str> {
  if hash.is_empty() {
    return Err(Error::EmptyHash);
  }
  if !hash
    .bytes()
    .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
  {
    return Err(Error::BadHash(hash.to_owned()));
  }
  Ok(hash)
}
