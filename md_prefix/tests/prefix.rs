//! 行首前缀拆分与还原

use md_prefix::{Part, restore};

/// 结构标记与空白拆入前缀，正文不含标记，拼回后与原文一致
#[test]
fn split_markup() {
  let cases = [
    ("  ## Title", "  ## ", "Title"),
    ("> quote", "> ", "quote"),
    (">> quote", ">> ", "quote"),
    ("> > quote", "> > ", "quote"),
    ("- item", "- ", "item"),
    ("+ item", "+ ", "item"),
    ("* item", "* ", "item"),
    ("1. item", "1. ", "item"),
    ("1) item", "1) ", "item"),
    ("> - nested", "> - ", "nested"),
    ("- [ ] task", "- [ ] ", "task"),
    ("- [x] task", "- [x] ", "task"),
    ("- [X] task", "- [X] ", "task"),
    ("###", "###", ""),
    ("1.", "1.", ""),
  ];
  for (line, prefix, body) in cases {
    let part = Part::new(line, true);
    assert_eq!((part.prefix, part.body), (prefix, body), "{line}");
    assert_eq!(restore(part.prefix, part.body), line);
  }
}

/// 非结构标记不拆：标签、负数、小数、强调、引号、中文标点
#[test]
fn keep_plain() {
  let cases = [
    ("#tag", "#tag"),
    ("####### title", "####### title"),
    ("-1", "-1"),
    ("+1", "+1"),
    ("1.0", "1.0"),
    ("*强调*", "*强调*"),
    ("_强调_", "_强调_"),
    ("\"引号\"", "\"引号\""),
    ("'quote'", "'quote'"),
    ("，文本", "，文本"),
    ("| 表格 |", "| 表格 |"),
    ("[链接](url)", "[链接](url)"),
  ];
  for (line, body) in cases {
    let part = Part::new(line, true);
    assert_eq!((part.prefix, part.body), ("", body), "{line}");
    assert_eq!(restore(part.prefix, part.body), line);
  }
}

/// 只拆行首空白，行尾空白始终去掉
#[test]
fn whitespace() {
  let part = Part::new("    text  ", true);
  assert_eq!((part.prefix, part.body), ("    ", "text"));
  assert_eq!(restore(part.prefix, part.body), "    text");

  let part = Part::new("  ", true);
  assert_eq!((part.prefix, part.body), ("", ""));
}

/// 代码块内只拆缩进，不拆 Markdown 标记
#[test]
fn no_markdown() {
  let cases = [
    ("## Title", "", "## Title"),
    ("  ## Title", "  ", "## Title"),
    ("- [ ] task", "", "- [ ] task"),
    ("> quote", "", "> quote"),
    ("    1. code", "    ", "1. code"),
  ];
  for (line, prefix, body) in cases {
    let part = Part::new(line, false);
    assert_eq!((part.prefix, part.body), (prefix, body), "{line}");
    assert_eq!(restore(part.prefix, part.body), line);
  }
}

/// 边界：空串、纯标记、CRLF、多字节贴边、数量上限
#[test]
fn edge() {
  // 空串与纯空白行
  for line in ["", " ", "\t", "\r\n"] {
    let part = Part::new(line, true);
    assert_eq!((part.prefix, part.body), ("", ""), "{line:?}");
  }
  // 标记后即行尾，整体入前缀
  for line in [
    ">", ">>>", "#", "######", "-", "+", "*", "1.", "9)", "- [ ]", "- [x]",
  ] {
    let part = Part::new(line, true);
    assert_eq!((part.prefix, part.body), (line, ""), "{line}");
  }
  // CRLF：行尾 \r\n 随 trim_end 去掉
  let part = Part::new("- item\r\n", true);
  assert_eq!((part.prefix, part.body), ("- ", "item"));
  // 多字节贴边、上限、无空白标记
  let cases = [
    ("#中文", "", "#中文"),
    ("# 中文", "# ", "中文"),
    ("- 中文", "- ", "中文"),
    ("- [ ] 中文", "- [ ] ", "中文"),
    ("+ [ ] t", "+ [ ] ", "t"),
    ("* [x] t", "* [x] ", "t"),
    ("中文。", "", "中文。"),
    ("中文  ", "", "中文"),
    ("\u{3000}中文", "\u{3000}", "中文"),
    ("\u{3000}# 中文", "\u{3000}# ", "中文"),
    (">>引用", "", ">>引用"),
    ("- [ ]x", "- ", "[ ]x"),
    ("###### 6 级", "###### ", "6 级"),
    ("####### 7 级", "", "####### 7 级"),
    ("0. 零", "0. ", "零"),
    ("999999999. 顶格", "999999999. ", "顶格"),
    ("1000000000. 溢出", "", "1000000000. 溢出"),
  ];
  for (line, prefix, body) in cases {
    let part = Part::new(line, true);
    assert_eq!((part.prefix, part.body), (prefix, body), "{line}");
    assert_eq!(restore(part.prefix, part.body), line.trim_end());
  }
}
