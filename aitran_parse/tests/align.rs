//! 原文与译文分别解析后按行对齐

use aitran_parse::Txt;

#[test]
fn align_source_and_translation() {
  let src_li = ["  ## Title", "Hello", "", "...", "World"];
  let dst_li = ["  ## 标题", "你好", "", "...", "世界"];
  let a = Txt::new(&src_li);
  let b = Txt::new(&dst_li);
  assert_eq!(a.line_li.len(), b.line_li.len());
  // 行号与待译位一一对应
  for (x, y) in a.line_li.iter().zip(&b.line_li) {
    assert_eq!(x.nth, y.nth);
  }
  assert_eq!(b.line_li[0].prefix, "  ## ");

  // 模拟用户修改译文第 5 行
  let user_li = ["  ## 标题", "你好", "", "...", "世界啊"];
  let c = Txt::new(&user_li);
  let changed_li: Vec<usize> = b
    .line_li
    .iter()
    .zip(&c.line_li)
    .enumerate()
    .filter(|(_, (x, y))| x.body != y.body)
    .map(|(i, _)| i)
    .collect();
  assert_eq!(changed_li, [2]);
}
