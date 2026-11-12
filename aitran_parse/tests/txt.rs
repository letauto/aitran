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

#[test]
fn fence_unclosed() {
  // 未闭合围栏延续到文末，内部纯符号行也送模型
  let src_li = ["```js", "});", "code"];
  let txt = Txt::new(&src_li);
  assert_eq!(txt.line_li.len(), 3);
  assert_eq!(txt.restore(["a", "b", "c"]), ["a", "b", "c"]);
}

#[test]
fn fence_nested() {
  // 反引号围栏内的波浪线行是代码内容，既不开新围栏也不关闭
  let src_li = ["```", "~~~js", "hello", "```", "..."];
  let txt = Txt::new(&src_li);
  assert_eq!(txt.line_li.len(), 4);
  assert_eq!(txt.line_li[1].body, "~~~js");
  assert_eq!(
    txt.restore(["a", "b", "c", "d"]),
    ["a", "b", "c", "d", "..."]
  );
}

#[test]
fn fence_close_rules() {
  // 更长的同字符围栏关闭；关闭行尾随内容则不关闭
  let src_li = ["```", "````", "x", "````", "y"];
  let txt = Txt::new(&src_li);
  assert_eq!(txt.line_li.len(), 5);

  let src_li = ["```", "``` tail", "x", "```"];
  let txt = Txt::new(&src_li);
  assert_eq!(txt.line_li.len(), 4);

  // 关闭围栏短于开启长度不算关闭，后续行仍是代码
  let src_li = ["````", "```", "x", "````", "y"];
  let txt = Txt::new(&src_li);
  assert_eq!(txt.line_li.len(), 5);
  assert_eq!(
    txt.restore(["a", "b", "c", "d", "y"]),
    ["a", "b", "c", "d", "y"]
  );
}

#[test]
fn fence_indent_boundary() {
  // 缩进 3 个空格仍是围栏，开启与关闭同样规则；围栏内只拆缩进
  let src_li = ["   ~~~", "x", "   ~~~", "..."];
  let txt = Txt::new(&src_li);
  assert_eq!(txt.line_li.len(), 3);
  assert_eq!(txt.line_li[0].prefix, "   ");
  assert_eq!(txt.line_li[0].body, "~~~");
  assert_eq!(txt.restore(["a", "b", "c"]), ["   a", "b", "   c", "..."]);
}

#[test]
fn fence_inline_code() {
  // 信息串含反引号是行内代码不是围栏
  let src_li = ["```a```", "x", "```"];
  let txt = Txt::new(&src_li);
  assert_eq!(txt.line_li.len(), 3);
  assert_eq!(txt.line_li[0].body, "```a```");
}

#[test]
fn fence_indent() {
  // 缩进 4 个空格不算围栏，只当普通行拆前缀
  let src_li = ["    ```js", "x", "```"];
  let txt = Txt::new(&src_li);
  assert_eq!(txt.line_li.len(), 3);
  assert_eq!(txt.line_li[0].prefix, "    ");
  assert_eq!(txt.line_li[0].body, "```js");
}

#[test]
fn fence_no_md_marks() {
  // 围栏内只拆缩进，- 与 # 不当 Markdown 标记
  let src_li = ["```", "- item", "# head", "```"];
  let txt = Txt::new(&src_li);
  assert_eq!(txt.line_li[1].prefix, "");
  assert_eq!(txt.line_li[1].body, "- item");
  assert_eq!(txt.line_li[2].body, "# head");
}

#[test]
fn fence_tilde() {
  let src_li = ["~~~", "x", "~~~", "..."];
  let txt = Txt::new(&src_li);
  assert_eq!(txt.line_li.len(), 3);
}

#[test]
fn crlf() {
  // 行尾 \r 随行尾空白一起去掉，直通行原样保留也不含 \r
  let src_li = ["Title\r", "\r", "...\r"];
  let txt = Txt::new(&src_li);
  assert_eq!(txt.line_li.len(), 1);
  assert_eq!(txt.line_li[0].body, "Title");
  assert_eq!(txt.restore(["标题"]), ["标题", "", "..."]);
}

#[test]
fn restore_extra_ignores() {
  // 译文多于待译行时忽略多余
  let src_li = ["Hello", "..."];
  let txt = Txt::new(&src_li);
  assert_eq!(txt.restore(["你好", "多余"]), ["你好", "..."]);
}
