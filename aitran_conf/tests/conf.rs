use std::{
  fs,
  path::{Path, PathBuf},
};

use aitran_conf::{Conf, Error, Task, from_str, load_from};
use aitran_lang::Code;
use strum::VariantArray;

/// 用例目录
const CASE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/case");

fn case(name: &str) -> PathBuf {
  Path::new(CASE).join(name)
}

fn read(name: &str) -> String {
  fs::read_to_string(case(name)).unwrap()
}

/// 解析单个用例文件
fn case_conf(name: &str) -> Conf {
  from_str(&read(name)).unwrap().unwrap()
}

/// 解析只含一个任务的用例
fn case_task(name: &str) -> Task {
  case_conf(name).task_li.into_iter().next().unwrap()
}

#[test]
fn parse_sample() {
  let task = case_task("sample.nt");
  assert_eq!(task.dir, Path::new("./i18n"));
  assert_eq!(task.from, Code::Zh);
  assert_eq!(task.from.name(), "简体中文");
  assert_eq!(task.to_li, [Code::En, Code::ZhTW]);
  assert_eq!(
    task
      .to_li
      .iter()
      .map(|code| code.name())
      .collect::<Vec<_>>(),
    ["English", "正體中文"]
  );
}

#[test]
fn empty_to_means_all() {
  let task = case_task("empty_to.nt");
  assert_eq!(task.dir, Path::new("./i18n"));
  assert_eq!(task.to_li.len(), Code::VARIANTS.len() - 1);
  assert!(!task.to_li.contains(&task.from));
  // 源语言只排除本变体，繁中仍在
  assert!(task.to_li.contains(&Code::ZhTW));
}

#[test]
fn missing_to_means_all() {
  let task = case_task("missing_to.nt");
  assert_eq!(task.to_li.len(), Code::VARIANTS.len() - 1);
  assert!(!task.to_li.contains(&Code::En));
}

#[test]
fn dedupe_to() {
  let text = "\
-
  dir: ./i18n
  from: zh
  to:
    - en
    - en
    - zh
    - zh-TW
";
  let conf = from_str(text).unwrap().unwrap();
  // 重复项合并，源语言排除，保持首次出现顺序
  assert_eq!(conf.task_li[0].to_li, [Code::En, Code::ZhTW]);
}

#[test]
fn variant_lang_by_code() {
  // 变体名字按 by_code 归并后比较，排除的是同一个 Code
  let task = case_task("variant_zh_cn.nt");
  assert_eq!(task.from, Code::Zh);
  assert!(!task.to_li.contains(&Code::Zh));
  assert!(task.to_li.contains(&Code::ZhTW));

  let task = case_task("variant_zh_tw.nt");
  assert_eq!(task.from, Code::ZhTW);
  assert_eq!(task.from.name(), "正體中文");
  assert!(task.to_li.contains(&Code::Zh));
}

#[test]
fn multiple_task() {
  let conf = case_conf("multiple_task.nt");
  assert_eq!(conf.task_li.len(), 2);
  assert_eq!(conf.task_li[1].dir, Path::new("b"));
  assert_eq!(conf.task_li[1].from, Code::En);
  assert_eq!(conf.task_li[1].to_li, [Code::Zh]);
}

#[test]
fn unknown_lang() {
  let err = from_str(&read("unknown_lang.nt")).unwrap_err();
  assert!(matches!(err, Error::Lang(_)));
}

#[test]
fn missing_from_skip() {
  let conf = case_conf("missing_from.nt");
  // 缺少 from 的任务被忽略，只保留第二个
  assert_eq!(conf.task_li.len(), 1);
  assert_eq!(conf.task_li[0].dir, Path::new("keep"));
  assert_eq!(conf.task_li[0].from, Code::En);
}

#[test]
fn missing_dir_skip() {
  let conf = case_conf("missing_dir.nt");
  // 缺少 dir 的任务被忽略，只保留第二个
  assert_eq!(conf.task_li.len(), 1);
  assert_eq!(conf.task_li[0].dir, Path::new("keep"));
  assert_eq!(conf.task_li[0].from, Code::En);
}

#[test]
fn empty_and_comment() {
  assert!(from_str("").unwrap().is_none());
  assert!(from_str(&read("comment_only.nt")).unwrap().is_none());
  assert!(from_str("# a\r\n  # b\r\n").unwrap().is_none());
  assert!(from_str("\u{feff}# bom\n").unwrap().is_none());
}

#[test]
fn bom_text_parsed() {
  // 带 BOM 的非空配置同样要能解析，BOM 在读入处剥掉
  let text = "\
\u{feff}-
  dir: ./i18n
  from: zh
  to:
    - en
";
  let conf = from_str(text).unwrap().unwrap();
  assert_eq!(conf.task_li[0].from, Code::Zh);
  assert_eq!(conf.task_li[0].to_li, [Code::En]);
}

#[test]
fn dup_key_err() {
  assert!(matches!(
    from_str("key: a\nkey: b\n").unwrap_err(),
    Error::Parse(_)
  ));
}

#[test]
fn load_missing_file() {
  assert!(load_from(&case("missing.nt")).unwrap().is_none());
}

#[test]
fn load_file() {
  let conf = load_from(&case("sample.nt")).unwrap().unwrap();
  assert_eq!(conf.task_li[0].to_li, [Code::En, Code::ZhTW]);
}
