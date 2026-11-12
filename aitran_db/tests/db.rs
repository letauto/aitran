use std::{fs, path::PathBuf, rc::Rc};

use aitran_db::{Db, Error};
use compio::runtime::spawn;

const H: &str = "h";

fn db_path(dir: &tempfile::TempDir) -> PathBuf {
  dir.path().join("i18n.aii18n")
}

fn lines(v: &[&str]) -> Vec<String> {
  v.iter().map(|s| (*s).to_owned()).collect()
}

async fn set<S: AsRef<str>>(
  db: &Db,
  path: &str,
  lang: &str,
  line_li: &[S],
) -> aitran_db::Result<()> {
  db.set(path, lang, H, line_li).await
}

#[compio::test]
async fn set_get_roundtrip() {
  let dir = tempfile::tempdir().unwrap();
  let db = Db::open(db_path(&dir)).unwrap();

  set(&db, "src/a.md", "zh", &["你好", "世界"]).await.unwrap();
  set(&db, "src/a.md", "en", &["hello"]).await.unwrap();
  set(&db, "src/b.md", "zh", &["第二个"]).await.unwrap();
  // path 含多个分隔符：头行按最后两个 `/` 拆，path 原样往返
  set(&db, "deep/dir/c.md", "zh", &["深层"]).await.unwrap();

  assert_eq!(
    db.get("src/a.md", "zh").await.unwrap(),
    lines(&["你好", "世界"])
  );
  assert_eq!(db.get("src/a.md", "en").await.unwrap(), lines(&["hello"]));
  assert_eq!(db.get("src/b.md", "zh").await.unwrap(), lines(&["第二个"]));
  assert_eq!(
    db.get("deep/dir/c.md", "zh").await.unwrap(),
    lines(&["深层"])
  );
  assert!(db.get("src/a.md", "ja").await.unwrap().is_empty());
  assert!(db.get("nope.md", "zh").await.unwrap().is_empty());
}

#[compio::test]
async fn last_write_wins() {
  let dir = tempfile::tempdir().unwrap();
  let db = Db::open(db_path(&dir)).unwrap();

  set(&db, "a.md", "zh", &["old1", "old2"]).await.unwrap();
  set(&db, "a.md", "zh", &["new"]).await.unwrap();
  assert_eq!(db.get("a.md", "zh").await.unwrap(), lines(&["new"]));

  // 空内容也是有效的覆盖
  set(&db, "a.md", "zh", &[] as &[&str]).await.unwrap();
  assert!(db.get("a.md", "zh").await.unwrap().is_empty());
}

#[compio::test]
async fn get_with_off_tracks_record_order() {
  let dir = tempfile::tempdir().unwrap();
  let db = Db::open(db_path(&dir)).unwrap();

  assert!(db.get_with_off("a.md", "zh").await.unwrap().is_none());
  set(&db, "a.md", "zh", &["旧"]).await.unwrap();
  let old = db.get_with_off("a.md", "zh").await.unwrap().unwrap();
  set(&db, "a.md", "zh", &["新"]).await.unwrap();
  let new = db.get_with_off("a.md", "zh").await.unwrap().unwrap();

  assert_eq!(old.line_li, lines(&["旧"]));
  assert_eq!(old.hash, H);
  assert_eq!(new.line_li, lines(&["新"]));
  assert!(new.off > old.off);
}

#[compio::test]
async fn get_by_hash() {
  let dir = tempfile::tempdir().unwrap();
  let db = Db::open(db_path(&dir)).unwrap();

  db.set("a.md", "zh", "old", &["旧"]).await.unwrap();
  db.set("a.md", "zh", "new", &["新"]).await.unwrap();
  assert_eq!(
    db.get_hash("a.md", "zh", "old")
      .await
      .unwrap()
      .unwrap()
      .line_li,
    lines(&["旧"])
  );
  assert_eq!(
    db.get_hash("a.md", "zh", "new")
      .await
      .unwrap()
      .unwrap()
      .line_li,
    lines(&["新"])
  );
  assert!(db.get_hash("a.md", "zh", "none").await.unwrap().is_none());
  assert_eq!(
    db.get_hash_any("new", "zh").await.unwrap().unwrap().line_li,
    lines(&["新"])
  );
  assert!(db.get_hash_any("none", "zh").await.unwrap().is_none());
}

#[compio::test]
async fn compact_with_first_puts_source_first() {
  let dir = tempfile::tempdir().unwrap();
  let db = Db::open(db_path(&dir)).unwrap();

  set(&db, "a.md", "zh", &["a-zh"]).await.unwrap();
  set(&db, "a.md", "en", &["a-en"]).await.unwrap();
  set(&db, "a.md", "ja", &["a-ja"]).await.unwrap();
  db.compact_with_first(&["a.md"], Some("zh")).await.unwrap();

  let text = fs::read_to_string(db.path()).unwrap();
  assert_eq!(
    text,
    "@a.md/zh/h\n>a-zh\n@a.md/en/h\n>a-en\n@a.md/ja/h\n>a-ja\n"
  );
  let src = db.get_with_off("a.md", "zh").await.unwrap().unwrap();
  for lang in ["en", "ja"] {
    let record = db.get_with_off("a.md", lang).await.unwrap().unwrap();
    assert!(record.off > src.off);
  }
  assert_eq!(
    db.get_hash("a.md", "zh", "h")
      .await
      .unwrap()
      .unwrap()
      .line_li,
    lines(&["a-zh"])
  );
}

#[compio::test]
async fn compact_drops_translation_before_source() {
  let dir = tempfile::tempdir().unwrap();
  let db = Db::open(db_path(&dir)).unwrap();

  set(&db, "a.md", "zh", &["旧"]).await.unwrap();
  set(&db, "a.md", "en", &["old"]).await.unwrap();
  set(&db, "a.md", "zh", &["新"]).await.unwrap();
  db.compact_with_first(&["a.md"], Some("zh")).await.unwrap();

  assert!(db.get("a.md", "en").await.unwrap().is_empty());
  assert_eq!(db.get("a.md", "zh").await.unwrap(), lines(&["新"]));
}

#[compio::test]
async fn prefix_lines_roundtrip() {
  let dir = tempfile::tempdir().unwrap();
  let db = Db::open(db_path(&dir)).unwrap();

  let content = ["|开头的行", ">开头的行", "@开头的行", "", " 空格 "];
  set(&db, "a.md", "zh", &content).await.unwrap();
  assert_eq!(db.get("a.md", "zh").await.unwrap(), lines(&content));
}

#[compio::test]
async fn garbage_lines_ignored() {
  let dir = tempfile::tempdir().unwrap();
  let path = db_path(&dir);
  fs::write(&path, "@a.md/zh/h\r\n# 注释\r\n>第一行\r\n乱码\r\n>第二行").unwrap();

  let db = Db::open(&path).unwrap();
  assert_eq!(
    db.get("a.md", "zh").await.unwrap(),
    lines(&["第一行", "第二行"])
  );
}

#[compio::test]
async fn long_line_cross_buffer() {
  let dir = tempfile::tempdir().unwrap();
  let db = Db::open(db_path(&dir)).unwrap();
  let long = "x".repeat(200 * 1024);
  let content = vec![long.clone(), "tail".to_owned()];
  set(&db, "big.md", "zh", &content).await.unwrap();
  assert_eq!(db.get("big.md", "zh").await.unwrap(), content);
}

#[compio::test]
async fn invalid_input_rejected() {
  let dir = tempfile::tempdir().unwrap();
  let db = Db::open(db_path(&dir)).unwrap();

  assert!(matches!(
    set(&db, "", "zh", &["x"]).await,
    Err(Error::EmptyPath)
  ));
  assert!(matches!(
    set(&db, "a", "", &["x"]).await,
    Err(Error::EmptyLang)
  ));
  assert!(matches!(
    set(&db, "a", "z/h", &["x"]).await,
    Err(Error::BadLang(_))
  ));
  assert!(matches!(
    db.set("a", "zh", "", &["x"]).await,
    Err(Error::EmptyHash)
  ));
  assert!(matches!(
    db.set("a", "zh", "h/h", &["x"]).await,
    Err(Error::BadHash(_))
  ));
  assert!(matches!(
    set(&db, "a\nb", "zh", &["x"]).await,
    Err(Error::BadPath(_))
  ));
  assert!(matches!(
    set(&db, "a", "zh", &["x\ny"]).await,
    Err(Error::BadLine)
  ));
}

#[compio::test]
async fn compact_all_sorts_and_dedupes() {
  let dir = tempfile::tempdir().unwrap();
  let db = Db::open(db_path(&dir)).unwrap();

  set(&db, "b.md", "zh", &["b-old"]).await.unwrap();
  set(&db, "a.md", "zh", &["a-old"]).await.unwrap();
  set(&db, "b.md", "zh", &["b-new"]).await.unwrap();
  set(&db, "a.md", "en", &["a-en"]).await.unwrap();

  db.compact(&["a.md", "b.md"]).await.unwrap();

  let text = fs::read_to_string(db.path()).unwrap();
  assert_eq!(
    text,
    "@a.md/en/h\n>a-en\n@a.md/zh/h\n>a-old\n@b.md/zh/h\n>b-new\n"
  );
  assert_eq!(db.get("b.md", "zh").await.unwrap(), lines(&["b-new"]));

  // 压实后追加必须接在文件末尾，不能覆盖已压实内容
  set(&db, "c.md", "zh", &["c"]).await.unwrap();
  assert_eq!(db.get("a.md", "en").await.unwrap(), lines(&["a-en"]));
  assert_eq!(db.get("a.md", "zh").await.unwrap(), lines(&["a-old"]));
  assert_eq!(db.get("b.md", "zh").await.unwrap(), lines(&["b-new"]));
  assert_eq!(db.get("c.md", "zh").await.unwrap(), lines(&["c"]));
}

#[compio::test]
async fn compact_subset_drops_others() {
  let dir = tempfile::tempdir().unwrap();
  let db = Db::open(db_path(&dir)).unwrap();

  set(&db, "a.md", "zh", &["a1"]).await.unwrap();
  set(&db, "z.md", "zh", &["z1"]).await.unwrap();
  set(&db, "a.md", "zh", &["a2"]).await.unwrap();
  set(&db, "z.md", "zh", &["z2"]).await.unwrap();

  db.compact(&["a.md"]).await.unwrap();

  let text = fs::read_to_string(db.path()).unwrap();
  // 只保留白名单路径，z.md 全部删除
  assert_eq!(text, "@a.md/zh/h\n>a2\n");
  assert_eq!(db.get("a.md", "zh").await.unwrap(), lines(&["a2"]));
  assert!(db.get("z.md", "zh").await.unwrap().is_empty());
}

#[compio::test]
async fn index_follows_compact() {
  let dir = tempfile::tempdir().unwrap();
  let db = Db::open(db_path(&dir)).unwrap();

  set(&db, "a.md", "zh", &["v1"]).await.unwrap();
  set(&db, "b.md", "zh", &["x"]).await.unwrap();
  // 首次读取建立索引
  assert_eq!(db.get("a.md", "zh").await.unwrap(), lines(&["v1"]));
  // 索引建立后追加
  set(&db, "a.md", "zh", &["v2"]).await.unwrap();
  assert_eq!(db.get("a.md", "zh").await.unwrap(), lines(&["v2"]));

  // 压实重写偏移，索引必须同步
  db.compact(&["a.md", "b.md"]).await.unwrap();
  assert_eq!(db.get("a.md", "zh").await.unwrap(), lines(&["v2"]));
  assert_eq!(db.get("b.md", "zh").await.unwrap(), lines(&["x"]));

  // 压实后追加
  set(&db, "b.md", "zh", &["y"]).await.unwrap();
  assert_eq!(db.get("a.md", "zh").await.unwrap(), lines(&["v2"]));
  assert_eq!(db.get("b.md", "zh").await.unwrap(), lines(&["y"]));
}

#[compio::test]
async fn compact_many_records_cross_buffer() {
  let dir = tempfile::tempdir().unwrap();
  let db = Db::open(db_path(&dir)).unwrap();

  let path_li: Vec<String> = (0..2000).map(|i| format!("dir{i:04}.md")).collect();
  // 足够多的记录，确保扫描跨越多个 64 KiB 读缓冲
  for i in (0..2000).rev() {
    let path = format!("dir{i:04}.md");
    set(&db, &path, "zh", &["旧"]).await.unwrap();
  }
  for i in 0..2000 {
    let path = format!("dir{i:04}.md");
    set(&db, &path, "zh", &["新"]).await.unwrap();
  }
  db.compact(&path_li).await.unwrap();

  let text = fs::read_to_string(db.path()).unwrap();
  let head_li: Vec<&str> = text.lines().filter(|l| l.starts_with('@')).collect();
  assert_eq!(head_li.len(), 2000);
  assert_eq!(head_li[0], "@dir0000.md/zh/h");
  assert_eq!(head_li[1999], "@dir1999.md/zh/h");
  assert!(text.contains(">新") && !text.contains(">旧"));
  assert_eq!(db.get("dir1999.md", "zh").await.unwrap(), lines(&["新"]));
}

#[compio::test]
async fn compact_missing_file_is_noop() {
  let dir = tempfile::tempdir().unwrap();
  let db = Db::open(db_path(&dir)).unwrap();
  db.compact::<&str>(&[]).await.unwrap();
  assert!(!db.path().exists());
}

#[compio::test]
async fn compact_empty_clears_db() {
  let dir = tempfile::tempdir().unwrap();
  let db = Db::open(db_path(&dir)).unwrap();

  set(&db, "a.md", "zh", &["a"]).await.unwrap();
  set(&db, "b.md", "zh", &["b"]).await.unwrap();
  db.compact::<&str>(&[]).await.unwrap();

  assert_eq!(fs::read_to_string(db.path()).unwrap(), "");
  assert!(db.get("a.md", "zh").await.unwrap().is_empty());
  assert!(db.get("b.md", "zh").await.unwrap().is_empty());
}

#[compio::test]
async fn concurrent_sets_do_not_interleave() {
  let dir = tempfile::tempdir().unwrap();
  let db = Rc::new(Db::open(db_path(&dir)).unwrap());

  let task_li: Vec<_> = (0..32)
    .map(|i| {
      let db = Rc::clone(&db);
      spawn(async move {
        let path = format!("k{i}.md");
        set(&db, &path, "zh", &["第一行", "第二行"]).await.unwrap();
      })
    })
    .collect();
  for task in task_li {
    task.await.unwrap();
  }

  for i in 0..32 {
    let path = format!("k{i}.md");
    assert_eq!(
      db.get(&path, "zh").await.unwrap(),
      lines(&["第一行", "第二行"])
    );
  }
}

#[compio::test]
async fn second_instance_rejected() {
  let dir = tempfile::tempdir().unwrap();
  let path = db_path(&dir);

  let db = Db::open(&path).unwrap();
  set(&db, "a.md", "zh", &["x"]).await.unwrap();
  assert!(matches!(Db::open(&path), Err(Error::Locked(_))));

  // 释放后可以重新打开
  drop(db);
  let db = Db::open(&path).unwrap();
  assert_eq!(db.get("a.md", "zh").await.unwrap(), lines(&["x"]));
}

#[compio::test]
async fn corrupt_file_does_not_panic() {
  let dir = tempfile::tempdir().unwrap();
  let path = db_path(&dir);
  // 截断的记录头、二进制垃圾、无分隔符的头、NUL 字节行混在合法记录前后
  fs::write(
    &path,
    b"\xff\xfe garbage\r\n@bad head\r\n@nodashes\n@a.md/z\r\n\x00\x01\r\n@a.md/zh/h\n>ok\n\x00tail\r\n>a2\n",
  )
  .unwrap();

  let db = Db::open(&path).unwrap();
  // 合法记录仍可读，行间垃圾被忽略
  assert_eq!(db.get("a.md", "zh").await.unwrap(), lines(&["ok", "a2"]));
  assert!(db.get("nodashes", "h").await.unwrap().is_empty());

  // 垃圾行不阻碍后续追加与读取
  set(&db, "b.md", "zh", &["新记录"]).await.unwrap();
  assert_eq!(db.get("b.md", "zh").await.unwrap(), lines(&["新记录"]));
}

#[compio::test]
async fn invalid_utf8_line_is_error_not_panic() {
  let dir = tempfile::tempdir().unwrap();
  let path = db_path(&dir);
  // 内容行含非法 UTF-8 字节：读取报错而非 panic
  fs::write(&path, b"@a.md/zh/h\n>\xff\xfe\n").unwrap();

  let db = Db::open(&path).unwrap();
  assert!(matches!(db.get("a.md", "zh").await, Err(Error::Utf8(_))));
}

#[compio::test]
async fn empty_file_behaves_as_empty_db() {
  let dir = tempfile::tempdir().unwrap();
  let path = db_path(&dir);
  fs::write(&path, "").unwrap();

  let db = Db::open(&path).unwrap();
  assert!(db.get("a.md", "zh").await.unwrap().is_empty());
  assert!(db.get_with_off("a.md", "zh").await.unwrap().is_none());
  db.compact::<&str>(&[]).await.unwrap();
  assert_eq!(fs::read_to_string(&path).unwrap(), "");

  // 空文件上追加仍可用
  set(&db, "a.md", "zh", &["首条"]).await.unwrap();
  assert_eq!(db.get("a.md", "zh").await.unwrap(), lines(&["首条"]));
}
