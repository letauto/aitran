//! 行拆分、过滤与还原用例

use aitran_parse::Txt;

#[test]
fn split() {
  let src_li = ["  ## Title  ", "```js", "});", "```", "", "...", "Hello"];
  let txt = Txt::new(&src_li);
  assert_eq!(txt.line_li.len(), 5);
  assert_eq!(txt.line_li[0].nth, 1);
  assert_eq!(txt.line_li[0].prefix, "  ## ");
  assert_eq!(txt.line_li[0].body, "Title");
  // 围栏行与围栏内纯符号行都要翻译
  assert_eq!(txt.line_li[1].body, "```js");
  assert_eq!(txt.line_li[2].body, "});");
  // 空行与围栏外的纯标点直通，不占待译位
  assert_eq!(txt.line_li[3].body, "```");
  assert_eq!(txt.line_li[4].nth, 7);
  assert_eq!(txt.line_li[4].body, "Hello");
}

#[test]
fn restore() {
  let src_li = ["  ## Title", "Hello", "", "...", "World"];
  let txt = Txt::new(&src_li);
  assert_eq!(
    txt.restore(["标题", "你好", "世界"]),
    ["  ## 标题", "你好", "", "...", "世界"]
  );
}

#[test]
fn restore_missing() {
  // 译文少于待译行时缺的行只留前缀
  let src_li = ["  ## Title", "Hello"];
  let txt = Txt::new(&src_li);
  assert_eq!(txt.restore(["标题"]), ["  ## 标题", ""]);
}

#[test]
fn restore_tail() {
  // 末尾直通行按原顺序补齐；纯空白行与空行一样去尾空白后为空
  let src_li = ["Hello", "", "  ", "..."];
  let txt = Txt::new(&src_li);
  assert_eq!(txt.restore(["你好"]), ["你好", "", "", "..."]);
}

#[test]
fn empty() {
  let txt = Txt::new(&[] as &[&str]);
  assert!(txt.line_li.is_empty());
  assert!(txt.restore(Vec::<String>::new()).is_empty());
}

#[test]
fn body_skips_empty() {
  let src_li = ["  ## Title", "", "  ", "...", "Hello"];
  let txt = Txt::new(&src_li);
  assert_eq!(txt.body_li().collect::<Vec<_>>(), ["Title", "Hello"]);
}

#[test]
fn restore_trims_body() {
  let src_li = ["Hello"];
  let txt = Txt::new(&src_li);
  assert_eq!(txt.restore(["  你好  "]), ["你好"]);
}

#[test]
fn owned_source() {
  // 支持 Vec<String> 等拥有所有权的行
  let src_li = [String::from("Hello"), String::from("...")];
  let txt = Txt::new(src_li.iter());
  assert_eq!(txt.line_li.len(), 1);
  assert_eq!(txt.restore(["你好"]), ["你好", "..."]);
}
