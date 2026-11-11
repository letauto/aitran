#![cfg_attr(docsrs, feature(doc_cfg))]

//! 批量翻译 Markdown 文件。

use std::{
  collections::{HashMap, HashSet},
  env::current_dir,
  fs,
  io::{ErrorKind, IsTerminal, Write, stdout},
  path::{Path, PathBuf},
};

mod error;

pub use ai_api::clean_text as clean;
use ai_api::{Flow, OnThink};
use aitran_api::OnPrompt;
use aitran_conf::Task;
use aitran_db::Db;
use aitran_parse::{Line, Txt};
use aitran_progress::{Progress, View, eta_text};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use clap_args::clap::Arg;
pub use error::{Error, Result};
use find_ext::FindExt;

/// 任务配置文件名
const CONF: &str = "aitran.nt";
/// 翻译缓存目录
const DB_DIR: &str = ".aitran";
/// 进入备用屏幕、清屏、隐藏光标
const TTY_ON: &str = "\x1b[?1049h\x1b[2J\x1b[H\x1b[?25l";
/// 灰色思考
const GRAY: &str = "\x1b[90m";
/// 重置颜色、显示光标、离开备用屏幕
const TTY_OFF: &str = "\x1b[0m\x1b[?25h\x1b[?1049l";
/// 提示词日志 target
const PROMPT_LOG: &str = "aitran::prompt";

/// 文件内容的 BLAKE3 base64url（无填充）hash
fn hash(text: &str) -> String {
  URL_SAFE_NO_PAD.encode(blake3::hash(text.as_bytes()).as_bytes())
}

/// 解析参数并执行全部翻译任务
pub async fn run() -> Result<()> {
  let Some(matches) = clap_args::parse!(|cmd| {
    cmd
      .arg(
        Arg::new("conf")
          .short('c')
          .long("conf")
          .value_name("PATH")
          .help("任务配置文件，默认向上查找 aitran.nt"),
      )
      .arg(
        Arg::new("cli-conf")
          .long("cli-conf")
          .value_name("PATH")
          .help("API 配置文件，默认 confdir/aii18n.nt"),
      )
  }) else {
    return Ok(());
  };
  let conf_path = match matches.get_one::<String>("conf") {
    Some(path) => PathBuf::from(path),
    None => find_conf::find_conf(CONF)?.ok_or(Error::NoConf)?,
  };
  let Some(conf) = aitran_conf::load_from(&conf_path)? else {
    return Err(Error::NoTask);
  };
  if conf.task.is_empty() {
    return Err(Error::NoTask);
  }
  let api_conf = match matches.get_one::<String>("cli-conf") {
    Some(path) => aitran_cli_conf::load_from::<aitran_cli_conf::Conf>(Path::new(path))?,
    None => aitran_cli_conf::load()?,
  };
  let Some(api_conf) = api_conf.filter(|conf| !conf.api.is_empty()) else {
    return Err(Error::NoApi);
  };

  let base = conf_path
    .parent()
    .filter(|path| !path.as_os_str().is_empty())
    .unwrap_or(Path::new("."));
  let base = if base.is_absolute() {
    base.to_path_buf()
  } else {
    current_dir()?.join(base)
  };
  let finder = FindExt::new(["md"]);
  let mut tasks = Vec::with_capacity(conf.task.len());
  let mut total_job = 0usize;
  let mut total_weight = 0u64;
  for mut task in conf.task {
    if task.dir.is_relative() {
      task.dir = base.join(&task.dir);
    }
    let src_dir = task.dir.join(task.from.as_str());
    let mut items = Vec::new();
    let mut weight = 0u64;
    for path in collect(&finder, &src_dir)? {
      let Ok(rel) = path.strip_prefix(&src_dir) else {
        continue;
      };
      let bytes = fs::metadata(&path)?.len();
      let key = rel.to_string_lossy().into_owned();
      let rel = rel.to_path_buf();
      weight = weight.saturating_add(bytes);
      items.push(Item {
        shown: show(&path, &base),
        path,
        rel,
        key,
        bytes,
      });
    }
    let n = task.to.len();
    total_job = total_job.saturating_add(items.len().saturating_mul(n));
    total_weight = total_weight.saturating_add(weight.saturating_mul(n as u64));
    tasks.push((task, items));
  }

  let mut progress = Progress::new(total_weight, total_job);
  // 同一目录只压实一次，避免多任务互相删掉记录
  let mut compact: HashMap<PathBuf, (String, Vec<String>)> = HashMap::new();
  for (task, items) in tasks {
    let cache_dir = task.dir.join(DB_DIR);
    fs::create_dir_all(&cache_dir)?;
    let db = Db::open(cache_dir.join(format!("{}.db", task.from.as_str())))?;
    let rel_li: Vec<String> = items.iter().map(|item| item.key.clone()).collect();

    // 清理源文件已删除、目标目录仍存在的译文
    prune_targets(&finder, &task, &items, &db).await?;

    // 先扫描全部目标文件，把用户修改追加回数据库，再注入原文
    for item in &items {
      for code in &task.to {
        let out = task.dir.join(code.as_str()).join(&item.rel);
        sync_tran(&db, &item.key, task.from.as_str(), code.as_str(), &out).await?;
      }
    }

    for item in &items {
      let src = fs::read_to_string(&item.path)?;
      let line_li: Vec<&str> = src.lines().collect();
      let txt = Txt::new(&line_li);
      let body_li: Vec<&str> = txt.body_li().collect();
      let src_rec = db.get_with_off(&item.key, task.from.as_str()).await?;
      let changed = src_rec.as_ref().map_or(!body_li.is_empty(), |record| {
        !same(&record.line_li, &txt.line_li)
      });
      let src_off = src_rec.as_ref().map(|record| record.off);
      let src_hash = changed.then(|| hash(&src));
      if let Some(src_hash) = &src_hash {
        db.set(&item.key, task.from.as_str(), src_hash, &body_li)
          .await?;
      }
      // 原文正文未变时沿用数据库里的 hash，空行/前缀变化不改变缓存关联
      let job_hash = src_hash.unwrap_or_else(|| {
        src_rec
          .as_ref()
          .map_or_else(String::new, |record| record.hash.clone())
      });
      for code in &task.to {
        let job = Job {
          api_li: &api_conf.api,
          db: &db,
          task: &task,
          lang: code.as_str(),
          shown: &item.shown,
          rel: &item.rel,
          key: &item.key,
          src: &src,
          line_li: &line_li,
          txt: &txt,
          changed,
          src_off,
          weight: item.bytes,
          hash: &job_hash,
        };
        translate(job, &mut progress).await?;
      }
    }
    compact
      .entry(db.path().to_path_buf())
      .or_insert_with(|| (task.from.as_str().to_owned(), Vec::new()))
      .1
      .extend(rel_li);
  }
  for (path, (first, rel_li)) in compact {
    Db::open(path)?
      .compact_with_first(&rel_li, Some(&first))
      .await?;
  }
  Ok(())
}

/// 一个待翻译源文件
struct Item {
  path: PathBuf,
  rel: PathBuf,
  key: String,
  shown: PathBuf,
  bytes: u64,
}

/// 单文件单目标语言的翻译上下文
struct Job<'a> {
  api_li: &'a [ai_api::Api],
  db: &'a Db,
  task: &'a Task,
  lang: &'a str,
  shown: &'a Path,
  rel: &'a Path,
  key: &'a str,
  src: &'a str,
  line_li: &'a [&'a str],
  txt: &'a Txt<'a>,
  changed: bool,
  src_off: Option<u64>,
  weight: u64,
  hash: &'a str,
}

/// 翻译单个文件到目标语言目录
async fn translate(job: Job<'_>, progress: &mut Progress) -> Result<()> {
  let Job {
    api_li,
    db,
    task,
    lang,
    shown,
    rel,
    key,
    src,
    line_li,
    txt,
    changed,
    src_off,
    weight,
    hash,
  } = job;
  let mut tran_rec = db.get_with_off(key, lang).await?;
  let mut from_hash = false;
  if tran_rec.is_none()
    && !hash.is_empty()
    && let Some(record) = db.get_hash_any(hash, lang).await?
    && record.line_li.len() == txt.line_li.len()
  {
    tran_rec = Some(record);
    from_hash = true;
  }
  let tran_li = tran_rec
    .as_ref()
    .map_or(&[][..], |record| record.line_li.as_slice());
  // 译文记录必须晚于当前原文记录；原文变更会追加新原文，旧译文自然失效
  let valid = from_hash
    || (!changed
      && tran_li.len() == txt.line_li.len()
      && src_off
        .is_some_and(|src_off| tran_rec.as_ref().is_some_and(|record| record.off > src_off)));
  let cache = if valid {
    cache_map(tran_li, txt)
  } else {
    HashMap::new()
  };

  let out_path = task.dir.join(lang).join(rel);
  // 全部命中缓存时没有思考输出，不进入备用屏幕
  let hit_all = txt.line_li.iter().all(|line| cache.contains_key(line.body));
  let mut think = Think::new();
  let mut reply = ();
  let mut prompt = LogPrompt;
  if hit_all {
    progress.skip(weight);
  } else {
    progress.start();
    let first = progress.done_job() == 0;
    think.begin(shown, lang, &progress.view(), first);
  }
  let got = {
    let hit = |body: &str| cache.get(body).map(|s| (*s).to_owned());
    aitran_api::tran_ask_hook_cache(
      api_li,
      task.from.as_str(),
      lang,
      line_li,
      txt,
      hit,
      aitran_api::Hooks {
        think: &mut think,
        reply: &mut reply,
        prompt: &mut prompt,
      },
    )
    .await
  };
  if !hit_all {
    think.finish();
  }
  let out = got?;

  // 只缓存待译行的译文，trim 后按原文顺序对齐
  let mut tran_out = Vec::with_capacity(txt.line_li.len());
  for line in &txt.line_li {
    let Some(full) = out.get(line.nth - 1) else {
      tran_out.push("");
      continue;
    };
    tran_out.push(full.strip_prefix(line.prefix).unwrap_or(full).trim());
  }
  let same_tran = tran_rec.as_ref().is_some_and(|record| {
    record
      .line_li
      .iter()
      .map(String::as_str)
      .eq(tran_out.iter().copied())
  });
  let cacheable = !tran_out.is_empty() && tran_out.iter().all(|tran| !tran.is_empty());
  if cacheable && (from_hash || !(valid && same_tran)) {
    db.set(key, lang, hash, &tran_out).await?;
  }

  if let Some(parent) = out_path.parent() {
    fs::create_dir_all(parent)?;
  }
  let mut text = out.join("\n");
  if src.ends_with('\n') {
    text.push('\n');
  }
  fs::write(&out_path, text)?;

  if !hit_all {
    progress.finish(weight);
  }
  let shown = Path::new(lang).join(rel);
  let shown = shown.to_string_lossy();
  let shown = clean(&shown);
  if hit_all {
    println!("→ {shown}");
  } else {
    println!("→ {shown} {}/{}", progress.done_job(), progress.total_job());
  }
  Ok(())
}

/// 待译正文与译文按行匹配后构建缓存
fn cache_map<'r, 't>(tran_rec: &'r [String], txt: &Txt<'t>) -> HashMap<&'t str, &'r str> {
  let mut map = HashMap::new();
  if tran_rec.len() != txt.line_li.len() {
    return map;
  }
  for (line, tran) in txt.line_li.iter().zip(tran_rec) {
    let tran = tran.trim();
    if !tran.is_empty() {
      map.insert(line.body, tran);
    }
  }
  map
}

/// 对比目标文件与译文缓存；用户改过且行数一致时追加新译文记录
async fn sync_tran(db: &Db, key: &str, from: &str, lang: &str, out: &Path) -> Result<()> {
  let target = match fs::read_to_string(out) {
    Ok(target) => target,
    Err(e) if e.kind() == ErrorKind::NotFound => return Ok(()),
    Err(e) => return Err(e.into()),
  };
  let target_line: Vec<&str> = target.lines().collect();
  let target_txt = Txt::new(&target_line);
  let Some(tran_rec) = db.get_with_off(key, lang).await? else {
    return Ok(());
  };
  // 正文未变，只有前缀、空行等变化时不更新
  if same(&tran_rec.line_li, &target_txt.line_li) {
    return Ok(());
  }
  let Some(src_rec) = db.get_hash(key, from, &tran_rec.hash).await? else {
    let path = out.to_string_lossy();
    eprintln!("aitran: 译文 {} 找不到匹配原文，跳过更新", clean(&path));
    return Ok(());
  };
  let body_li: Vec<&str> = target_txt.body_li().collect();
  if src_rec.line_li.len() != body_li.len() {
    let path = out.to_string_lossy();
    eprintln!(
      "aitran: 译文 {} 行数不一致 {} != {}，跳过更新",
      clean(&path),
      src_rec.line_li.len(),
      body_li.len()
    );
    return Ok(());
  }
  // 追加写入，和正常翻译保存使用同一路径
  db.set(key, lang, &tran_rec.hash, &body_li).await?;
  Ok(())
}

/// 原文记录是否与当前待译正文逐行一致
fn same(rec: &[String], line_li: &[Line]) -> bool {
  rec.len() == line_li.len() && rec.iter().zip(line_li).all(|(old, line)| old == line.body)
}

/// 收集目录下需要翻译的 markdown 文件，跳过目标语言输出目录
fn collect(finder: &FindExt, dir: &Path) -> Result<Vec<PathBuf>> {
  let mut out: Vec<PathBuf> = finder.find(dir)?.collect();
  out.sort_unstable();
  Ok(out)
}

/// 删除源目录不存在、目标目录仍存在的 markdown 文件
async fn prune_targets(finder: &FindExt, task: &Task, items: &[Item], db: &Db) -> Result<()> {
  let keep: HashSet<&Path> = items.iter().map(|item| item.rel.as_path()).collect();
  for code in &task.to {
    let dir = task.dir.join(code.as_str());
    if !dir.is_dir() {
      continue;
    }
    for path in collect(finder, &dir)? {
      let Ok(rel) = path.strip_prefix(&dir) else {
        continue;
      };
      if keep.contains(rel) {
        continue;
      }
      let key = rel.to_string_lossy();
      if db.get_with_off(&key, code.as_str()).await?.is_none() {
        continue;
      }
      match fs::remove_file(&path) {
        Ok(()) => {
          let shown = show(&path, &task.dir);
          eprintln!(
            "aitran: 删除无源文件的译文 {}",
            clean(&shown.to_string_lossy())
          );
        }
        Err(e) if e.kind() == ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
      }
    }
  }
  Ok(())
}

/// 显示路径：配置目录相对路径，前缀 `./`
fn show(path: &Path, base: &Path) -> PathBuf {
  path
    .strip_prefix(base)
    .ok()
    .map_or_else(|| path.to_path_buf(), |rel| Path::new(".").join(rel))
}

/// 思考回调：流式输出并滚动，文件结束离开备用屏幕即清空
struct Think {
  tty: bool,
  screen: bool,
  wrote: bool,
  nl: bool,
}

impl Think {
  fn new() -> Self {
    Self {
      tty: stdout().is_terminal(),
      screen: false,
      wrote: false,
      nl: false,
    }
  }

  /// 显示总进度与 ETA；有模型调用时进入备用屏幕
  fn begin(&mut self, src: &Path, lang: &str, view: &View, first: bool) {
    let src = src.to_string_lossy();
    let src = clean(&src);
    let lang = clean(lang);
    let status = if first { String::new() } else { status(view) };
    self.screen = self.tty;
    self.wrote = false;
    self.nl = false;
    if self.screen {
      print!("{TTY_ON}{status}开始翻译 {src} → {lang}\n{GRAY}");
      let _ = stdout().flush();
    } else {
      eprintln!("{status}开始翻译 {src} → {lang}");
    }
  }

  /// 恢复光标并离开备用屏幕，思考内容随之清空
  fn finish(&mut self) {
    if self.screen {
      self.screen = false;
      print!("{TTY_OFF}");
      let _ = stdout().flush();
    }
  }
}

/// 进度两行：百分比 / 完成数、预估剩余时间
fn status(view: &View) -> String {
  let pct = format!("{}.{}%", view.permille / 10, view.permille % 10);
  format!(
    "[{pct}] {}/{}  \n估算剩余时间 {} \n",
    view.done_job,
    view.total_job,
    eta_text(view.eta),
  )
}

impl OnThink for Think {
  fn think(&mut self, s: &str) -> Flow {
    let s = clean(s);
    if self.tty {
      let mut out = stdout().lock();
      let _ = out.write_all(s.as_bytes());
      let _ = out.flush();
    } else {
      // 非终端场景写到 stderr，避免污染译文输出
      eprint!("{s}");
    }
    self.wrote = true;
    self.nl = s.ends_with('\n');
    Flow::Continue
  }

  fn end(&mut self, _think: &str) {
    // 非终端下思考与下一行进度之间补换行，终端由备用屏幕整体清理
    if !self.tty && self.wrote && !self.nl {
      eprintln!();
    }
    self.wrote = false;
    self.nl = false;
  }
}

/// 把系统提示与实际发送给模型的用户消息写入日志
struct LogPrompt;

impl OnPrompt for LogPrompt {
  fn system(&mut self, text: &str) {
    log::info!(target: PROMPT_LOG, "system:\n{}", clean(text));
  }

  fn ask(&mut self, text: &str) {
    log::info!(target: PROMPT_LOG, "user:\n{}", clean(text));
  }
}

#[cfg(test)]
mod test {
  use std::{fs, path::PathBuf};

  use aitran_conf::from_str;
  use aitran_db::Db;
  use find_ext::FindExt;

  use super::{Item, hash, prune_targets, sync_tran};

  #[compio::test]
  async fn sync_edited_translation() {
    let dir = tempfile::tempdir().unwrap();
    let db = Db::open(dir.path().join("zh.db")).unwrap();
    let src_hash = hash("你好\n世界\n");
    db.set("a.md", "zh", &src_hash, &["你好", "世界"])
      .await
      .unwrap();
    db.set("a.md", "en", &src_hash, &["Hello", "World"])
      .await
      .unwrap();
    db.compact_with_first(&["a.md"], Some("zh")).await.unwrap();
    db.set("b.md", "zh", &src_hash, &["你好", "世界"])
      .await
      .unwrap();
    db.set("b.md", "en", &src_hash, &["Hello", "World"])
      .await
      .unwrap();

    let out = dir.path().join("a.md");
    fs::write(&out, "Hi\nWorld\n").unwrap();
    sync_tran(&db, "a.md", "zh", "en", &out).await.unwrap();
    sync_tran(&db, "b.md", "zh", "en", &out).await.unwrap();
    assert_eq!(
      db.get("a.md", "en").await.unwrap(),
      ["Hi".to_owned(), "World".to_owned()]
    );
    assert_eq!(
      db.get("a.md", "zh").await.unwrap(),
      ["你好".to_owned(), "世界".to_owned()]
    );

    // 内容未再变化时不追加记录
    let off = db.get_with_off("a.md", "en").await.unwrap().unwrap().off;
    sync_tran(&db, "a.md", "zh", "en", &out).await.unwrap();
    assert_eq!(
      db.get_with_off("a.md", "en").await.unwrap().unwrap().off,
      off
    );

    // 行数不一致只告警，不更新
    fs::write(&out, "Hi\nWorld\nExtra\n").unwrap();
    sync_tran(&db, "a.md", "zh", "en", &out).await.unwrap();
    assert_eq!(
      db.get("a.md", "en").await.unwrap(),
      ["Hi".to_owned(), "World".to_owned()]
    );
  }

  #[compio::test]
  async fn sync_after_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("zh.db");
    let src_hash = hash("你好\n世界\n");
    {
      let db = Db::open(&path).unwrap();
      db.set("a.md", "zh", &src_hash, &["你好", "世界"])
        .await
        .unwrap();
      db.set("a.md", "en", &src_hash, &["Hello", "World"])
        .await
        .unwrap();
      db.compact_with_first(&["a.md"], Some("zh")).await.unwrap();
    }
    let db = Db::open(&path).unwrap();
    let out = dir.path().join("a.md");
    fs::write(&out, "Hi\nWorld\n").unwrap();
    sync_tran(&db, "a.md", "zh", "en", &out).await.unwrap();
    assert_eq!(
      db.get("a.md", "zh").await.unwrap(),
      ["你好".to_owned(), "世界".to_owned()]
    );
    assert_eq!(
      db.get("a.md", "en").await.unwrap(),
      ["Hi".to_owned(), "World".to_owned()]
    );
  }

  #[compio::test]
  async fn prune_deleted_sources() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    fs::create_dir_all(root.join("zh")).unwrap();
    fs::create_dir_all(root.join("en")).unwrap();
    fs::write(root.join("zh/a.md"), "你好\n").unwrap();
    fs::write(root.join("en/a.md"), "Hello\n").unwrap();
    fs::write(root.join("en/old.md"), "old\n").unwrap();
    let items = [Item {
      path: root.join("zh/a.md"),
      rel: PathBuf::from("a.md"),
      key: "a.md".to_owned(),
      shown: PathBuf::from("./zh/a.md"),
      bytes: 7,
    }];
    let mut task = from_str("-\n  dir: .\n  from: zh\n  to:\n    - en\n")
      .unwrap()
      .unwrap()
      .task
      .remove(0);
    task.dir = root.to_path_buf();
    let db = Db::open(root.join("zh.db")).unwrap();
    db.set("old.md", "en", "h", &["old"]).await.unwrap();
    prune_targets(&FindExt::new(["md"]), &task, &items, &db)
      .await
      .unwrap();
    assert!(root.join("en/a.md").exists());
    assert!(!root.join("en/old.md").exists());
  }
}
