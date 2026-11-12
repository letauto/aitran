//! 字符上限与分批相关用例

mod common;

use aitran_api::{CHAR_LIMIT, tran_with, tran_with_limit};
use aok::{OK, Void};
use common::{api, serve_rec, sse};

/// 两行字符数正好等于上限，合为一批
#[compio::test]
async fn tran_limit_exact() -> Void {
  let half = CHAR_LIMIT / 2;
  let a = "a".repeat(half);
  let b = "b".repeat(half);
  let (ca, cb) = (a.clone(), b.clone());
  let (addr, seen) =
    serve_rec(move |_, _| sse(&format!("1: {ca}\n中文: 甲\n-\n2: {cb}\n中文: 乙\n")));
  let api_li = [api(addr)];
  let src_li = [a.as_str(), b.as_str()];
  let out = tran_with(&api_li, "English", "中文", &src_li).await?;
  assert_eq!(out, ["甲", "乙"]);
  assert_eq!(seen.lock().unwrap().len(), 1);
  OK
}

/// 超限单行独占一批后，后续行另起一批，行号正确回填
#[compio::test]
async fn tran_limit_tail() -> Void {
  let long = "a".repeat(CHAR_LIMIT + 1);
  let tail = long.clone();
  let (addr, seen) = serve_rec(move |n, _| {
    if n == 0 {
      sse(&format!("1: {tail}\n中文: 甲\n"))
    } else {
      sse("1: b\n中文: 乙\n")
    }
  });
  let api_li = [api(addr)];
  let out = tran_with(&api_li, "English", "中文", &[long.as_str(), "b"]).await?;
  assert_eq!(out, ["甲", "乙"]);
  assert_eq!(seen.lock().unwrap().len(), 2);
  OK
}

/// 一批正好用完上限后，下一行另起一批，行号仍按原文回填
#[compio::test]
async fn tran_limit_exact_tail() -> Void {
  let half = CHAR_LIMIT / 2;
  let a = "a".repeat(half);
  let b = "b".repeat(half);
  let (ca, cb) = (a.clone(), b.clone());
  let (addr, seen) = serve_rec(move |n, _| {
    if n == 0 {
      sse(&format!("1: {ca}\n中文: 甲\n-\n2: {cb}\n中文: 乙\n"))
    } else {
      // 每批独立会话，批内行号从 1 起
      sse("1: c\n中文: 丙\n")
    }
  });
  let api_li = [api(addr)];
  let src_li = [a.as_str(), b.as_str(), "c"];
  let out = tran_with(&api_li, "English", "中文", &src_li).await?;
  assert_eq!(out, ["甲", "乙", "丙"]);
  assert_eq!(seen.lock().unwrap().len(), 2);
  OK
}

/// 字符数按字符而非字节统计，多字节内容不提前拆批
#[compio::test]
async fn tran_limit_multibyte() -> Void {
  let half = CHAR_LIMIT / 2;
  let a = "中".repeat(half);
  let b = "文".repeat(half);
  let (ca, cb) = (a.clone(), b.clone());
  let (addr, seen) =
    serve_rec(move |_, _| sse(&format!("1: {ca}\nEnglish: 甲\n-\n2: {cb}\nEnglish: 乙\n")));
  let api_li = [api(addr)];
  let src_li = [a.as_str(), b.as_str()];
  let out = tran_with(&api_li, "中文", "English", &src_li).await?;
  assert_eq!(out, ["甲", "乙"]);
  assert_eq!(seen.lock().unwrap().len(), 1);
  OK
}

/// 单行超过上限时独占一批，不从行中间切开
#[compio::test]
async fn tran_limit_single() -> Void {
  let a = "a".repeat(CHAR_LIMIT + 1);
  let ca = a.clone();
  let (addr, seen) = serve_rec(move |_, _| sse(&format!("1: {ca}\n中文: 甲\n")));
  let api_li = [api(addr)];
  let out = tran_with(&api_li, "English", "中文", &[a.as_str()]).await?;
  assert_eq!(out, ["甲"]);
  assert_eq!(seen.lock().unwrap().len(), 1);
  OK
}

/// 编译期常量泛型：小上限即可拆成两批，不必构造超大字符串
#[compio::test]
async fn tran_limit_small_batches() -> Void {
  let (addr, seen) = serve_rec(|n, _| {
    if n == 0 {
      sse("1: aaaaa\n中文: 甲\n")
    } else {
      sse("1: bbbbb\n中文: 乙\n")
    }
  });
  let api_li = [api(addr)];
  let out = tran_with_limit::<6>(&api_li, "English", "中文", &["aaaaa", "bbbbb"]).await?;
  assert_eq!(out, ["甲", "乙"]);
  assert_eq!(seen.lock().unwrap().len(), 2);
  OK
}
