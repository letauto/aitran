//! 提示词生成与回复解析
//!
//! system 提示只放角色、格式与规则；用户消息只放数据（行号 + 原文）。
//! 回复按单独一行 `-` 切块，每块三行字段：行、原文、译文。
//! 块内按非空行定位：首行行、次行原文、其余为译文；译文删掉所有换行并
//! trim，保证与原文字段一样只有一行。
//! 块格式错误或原文不一致由调用方触发下一轮修订。
//!
//! 提示词中的行号是批内 1 基序号，与批次切分无关；解析回的行号同样是
//! 批内序号，由调用方映射回待译行。

use std::borrow::Cow;

use aitran_lang::code_with_name_if_exist;
use aitran_parse::Line;
use itoa::Buffer;

/// 块分隔行
const SEP: &str = "-";
/// 代码围栏前缀
const FENCE: &str = "```";
/// 行首可剥离的 Markdown 标记
const MARK: [char; 5] = ['#', '>', '*', '_', '`'];
/// 行首可剥离的强调标记
const EMPH: [char; 3] = ['*', '_', '`'];
/// 字段分隔符（半角与全角冒号）
const COLON: [char; 2] = [':', '：'];
/// 首次请求头部容量估算
const ASK_HEAD: usize = 32;
/// 每条数据行的固定开销估算：标签、冒号、空格、换行
const LINE_FIX: usize = 24;
/// 修订请求头部容量估算
const FIX_HEAD: usize = 160;
/// 修订请求每条问题行的固定开销估算
const FIX_LINE: usize = 96;
/// 系统提示中格式演示的固定文本开销估算
const SYS_FIX: usize = 64;

/// 某一行未完成的原因
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Why {
  /// 回复里没有这一行
  Miss,
  /// 原文与实际不一致
  Orig,
  /// 译文为空
  Empty,
}

impl Why {
  const fn text(self) -> &'static str {
    match self {
      Self::Miss => "没有收到这一行",
      Self::Orig => "原文与输入不一致",
      Self::Empty => "译文为空",
    }
  }
}

/// 一对语言的提示词生成器
pub(crate) struct Prompt {
  /// 原文语言标签，句中使用
  from: String,
  /// 译文语言标签，句中使用
  to: String,
  /// 译文语言代码，字段中使用
  to_code: String,
}

impl Prompt {
  /// 指定翻译方向
  pub(crate) fn new(from: &str, to: &str) -> Self {
    Self {
      from: code_with_name_if_exist(from),
      to: code_with_name_if_exist(to),
      to_code: to.to_owned(),
    }
  }

  /// 系统提示：角色、输出格式与规则
  pub(crate) fn system(&self) -> String {
    const HEAD_A: &str = "你是专业翻译，正在把 ";
    const HEAD_B: &str = " 翻译为 ";
    const HEAD_C: &str =
      "。\n\n只做翻译，不输出解释\n\n输入每行是「行号: 原文」，输出每个翻译块之间用单独一行 ";
    const MID: &str = " 分隔
为每行补上译文，按下面格式输出，行号必须与输入一致：

";
    const RULE: &str = "

要求：
1. 每个块之后必须输出单独一行 ";
    const RULE_TAIL: &str = "
2. 每个字段一行，标签后写冒号再写值，不加空格，值里可以出现冒号（无需转义）
3. 译文保持单行，不加引号、编号、注释
4. 译文结尾标点与原文一致，原文结尾没有标点就不要加
5. 行内代码与模板变量（如 {{name}}、$var、%s、<tag>）保持原样，纯英文的不翻译，含其他语言的按上下文翻译
6. Markdown 围栏（``` 或 ~~~）内的代码只翻译注释（如 //、# 后的说明文字），变量名、函数名、关键字、字符串等保持原样，围栏行本身不翻译
7. 整个输出必须用三反引号代码围栏包裹，围栏外不写任何内容，围栏内只放这些块
8. 文风语气要和原文一致，译文要专业地道、信达雅";
    let Self { from, to, .. } = self;
    let cap = HEAD_A
      .len()
      .saturating_add(HEAD_B.len())
      .saturating_add(HEAD_C.len())
      .saturating_add(MID.len())
      .saturating_add(RULE.len())
      .saturating_add(RULE_TAIL.len())
      .saturating_add(from.len())
      .saturating_add(to.len())
      .saturating_add(self.to_code.len())
      .saturating_add(SYS_FIX);
    let mut out = String::with_capacity(cap);
    out.push_str(HEAD_A);
    out.push_str(from);
    out.push_str(HEAD_B);
    out.push_str(to);
    out.push_str(HEAD_C);
    out.push_str(SEP);
    out.push_str(MID);
    demo(&mut out, &self.to_code);
    out.push_str(RULE);
    out.push_str(SEP);
    out.push_str(RULE_TAIL);
    out
  }

  /// 首次请求：只放原文数据，行号按批内顺序 1 基
  pub(crate) fn ask(line_li: &[Line<'_>]) -> String {
    let cap = line_li.iter().fold(ASK_HEAD, |cap, line| {
      cap.saturating_add(line.body.len().saturating_add(LINE_FIX))
    });
    let mut out = String::with_capacity(cap);
    for (k, line) in line_li.iter().enumerate() {
      data(&mut out, k + 1, line.body);
    }
    out
  }

  /// 修订请求：问题行摘要 + 正确原文 + 上一轮错误译文 + 期望格式
  ///
  /// `pending_li` 是批内序号，`why_li` / `wrong_li` 与批内序号对齐。
  pub(crate) fn fix(
    &self,
    line_li: &[Line<'_>],
    pending_li: &[usize],
    why_li: &[Option<Why>],
    wrong_li: &[Option<String>],
    bad: bool,
  ) -> String {
    // 原文在摘要与待修订列表各出现一次，另算错误译文；溢出时回退为 0 让 String 自增
    let cap = pending_li.iter().fold(FIX_HEAD, |cap, &k| {
      let extra = line_li[k]
        .body
        .len()
        .saturating_mul(2)
        .saturating_add(FIX_LINE)
        .saturating_add(
          wrong_li
            .get(k)
            .and_then(Option::as_ref)
            .map_or(0, String::len),
        );
      cap.saturating_add(extra)
    });
    let mut out = String::with_capacity(cap);
    out.push_str("上面的回复需要修订：\n");
    for &k in pending_li {
      let reason = why_li[k].unwrap_or(Why::Miss);
      let src = line_li[k].body;
      out.push_str("- ");
      out.push_str(Buffer::new().format(k + 1));
      out.push(':');
      out.push_str(reason.text());
      out.push_str("，原文：");
      out.push_str(src);
      if let Some(w) = wrong_li.get(k).and_then(Option::as_ref) {
        out.push_str("，错误译文：");
        out.push_str(w);
      }
      out.push('\n');
    }
    if bad {
      out.push_str("- 回复中存在无法解析的格式块\n");
    }
    out.push_str("只重新输出上面这些行，格式与之前完全相同，不要重复已正确的行，不要输出解释\n");
    out.push_str("待修订行：\n");
    for &k in pending_li {
      data(&mut out, k + 1, line_li[k].body);
    }
    out.push_str("期望格式示例：\n");
    demo(&mut out, &self.to_code);
    out
  }

  /// 解析回复，返回可读块与是否存在格式错误
  ///
  /// 先剥掉整段外层代码围栏再解析，缺围栏时按全文解析。
  pub(crate) fn parse<'m>(&self, msg: &'m str) -> Reply<'m> {
    let msg = fenced(msg).unwrap_or(msg);
    let mut reply = Reply {
      block_li: Vec::new(),
      bad: false,
    };
    let mut begin = 0;
    let mut off = 0;
    for seg in msg.split_inclusive('\n') {
      let end = off;
      off += seg.len();
      if seg.trim() != SEP {
        continue;
      }
      take(&mut reply, &msg[begin..end], &self.to_code);
      begin = off;
    }
    take(&mut reply, &msg[begin..], &self.to_code);
    reply
  }
}

/// 追加一行字符串字段
fn field(out: &mut String, tag: &str, val: &str) {
  out.push_str(tag);
  out.push(':');
  out.push_str(val);
  out.push('\n');
}

/// 追加一条输入数据：`行号: 原文`
fn data(out: &mut String, nth: usize, src: &str) {
  field(out, Buffer::new().format(nth), src);
}

/// 追加格式演示
fn demo(out: &mut String, to: &str) {
  field(out, "<行号>", "原文");
  field(out, to, "译文");
  out.push_str(SEP);
}

/// 单个解析出的块：原文借用回复，译文单行借用、折行时独占
#[derive(Debug)]
pub(crate) struct Block<'a> {
  /// 批内 1 基行号
  pub(crate) nth: usize,
  /// 原文值
  pub(crate) src: &'a str,
  /// 译文值，已删除换行
  pub(crate) tran: Cow<'a, str>,
}

/// 回复解析结果
#[derive(Debug)]
pub(crate) struct Reply<'a> {
  /// 解析出的块，顺序与回复一致
  pub(crate) block_li: Vec<Block<'a>>,
  /// 是否存在无法解析的格式块
  pub(crate) bad: bool,
}

/// 取首个围栏行（含语言标记）与最后一个单独 ``` 行之间的内容
fn fenced(msg: &str) -> Option<&str> {
  let mut begin = None;
  let mut end = None;
  let mut off = 0;
  for seg in msg.split_inclusive('\n') {
    let start = off;
    off += seg.len();
    let line = seg.trim();
    match begin {
      // 结束围栏必须是单独一行 ```
      Some(_) => {
        if line == FENCE {
          end = Some(start);
        }
      }
      None if line.starts_with(FENCE) => begin = Some(off),
      None => {}
    }
  }
  Some(&msg[begin?..end?])
}

fn take<'a>(reply: &mut Reply<'a>, text: &'a str, to: &str) {
  if text.trim().is_empty() {
    return;
  }
  match block(text, to) {
    Some(b) => reply.block_li.push(b),
    None => reply.bad = true,
  }
}

/// 块解析：首行是 `行号: 原文`，次行是 `译文: 译文`，缺字段即视为格式错误
///
/// 行号前的寒暄自动跳过；译文行 key 必须是 `to`，译文取标签行到块尾的全部内容，
/// 删除所有换行并 trim，避免模型自行折行导致译文截断或多行。
fn block<'a>(text: &'a str, to: &str) -> Option<Block<'a>> {
  let mut nth = None;
  let mut src = None;
  let mut tran = None;
  let mut off = 0;
  for seg in text.split_inclusive('\n') {
    let line = seg.strip_suffix('\n').unwrap_or(seg);
    let start = off;
    off += seg.len();
    if line.trim().is_empty() {
      continue;
    }
    if nth.is_none() {
      if let Some((n, s)) = src_field(line) {
        nth = Some(n);
        src = Some(s);
      }
      continue;
    }
    let v = field_of(line, to)?;
    let rest = &text[start + line.len()..];
    // 常见单行译文直接借用，续行才分配
    tran = Some(if rest.trim().is_empty() {
      Cow::Borrowed(v)
    } else {
      let mut val = String::with_capacity(v.len() + rest.len());
      val.push_str(v);
      val.push_str(rest);
      val.retain(|c| c != '\n' && c != '\r');
      // 原地 trim，省一次分配
      let start = val.len() - val.trim_start().len();
      if start > 0 {
        val.drain(..start);
      }
      val.truncate(val.trim_end().len());
      Cow::Owned(val)
    });
    break;
  }
  Some(Block {
    nth: nth?,
    src: src?,
    tran: tran?,
  })
}

/// 匹配 `行号: 原文`，返回行号与原文
fn src_field(line: &str) -> Option<(usize, &str)> {
  let s = line.trim_start().trim_start_matches(MARK).trim_start();
  let len = s.bytes().take_while(u8::is_ascii_digit).count();
  if len == 0 {
    return None;
  }
  let rest = s[len..].trim_start_matches(EMPH).trim_start();
  let rest = rest.strip_prefix(COLON).map(str::trim_start)?;
  Some((s[..len].parse().ok()?, rest.trim_end()))
}

/// 匹配 `标签: 值`，返回去首尾空白的值；值里允许出现冒号
///
/// 标签前允许空白、Markdown 标题与强调符，标签后必须是分隔符，
/// 因此 `English` 不会误配 `Englishman`，`C#` 也能按原样匹配。
fn field_of<'a>(line: &'a str, name: &str) -> Option<&'a str> {
  if name.is_empty() {
    return None;
  }
  let s = line.trim_start().trim_start_matches(MARK).trim_start();
  if !s.get(..name.len())?.eq_ignore_ascii_case(name) {
    return None;
  }
  // 标签后紧跟的字符必须是分隔符，避免前缀误配
  let rest = &s[name.len()..];
  if !rest.is_empty()
    && !rest.starts_with(COLON)
    && !rest.starts_with(EMPH)
    && !rest.starts_with(|c: char| c.is_whitespace())
  {
    return None;
  }
  let rest = rest.trim_start_matches(EMPH).trim_start();
  let rest = rest.strip_prefix(COLON).map_or(rest, str::trim_start);
  Some(rest.trim_end())
}

#[cfg(test)]
mod test {
  use aitran_parse::Txt;

  use super::{Prompt, Why};

  fn prompt() -> Prompt {
    Prompt::new("English", "中文")
  }

  #[test]
  fn ask_compact() {
    let src_li = ["Hello", "", "World"];
    let txt = Txt::new(&src_li);
    assert_eq!(Prompt::ask(&txt.line_li), "1:Hello\n2:World\n");
    assert!(prompt().system().contains("行号: 原文"));
    assert!(prompt().system().contains("中文:译文\n-"));
  }

  #[test]
  fn parse_reply() {
    let msg = "\
好的，以下是翻译：

**1**: English:
**中文**: 英文冒号
-
2: World
中文: 时间: 12:00
";
    let reply = prompt().parse(msg);
    assert!(!reply.bad);
    assert_eq!(reply.block_li.len(), 2);
    assert_eq!(reply.block_li[0].nth, 1);
    assert_eq!(reply.block_li[0].src, "English:");
    assert_eq!(&*reply.block_li[0].tran, "英文冒号");
    assert_eq!(&*reply.block_li[1].tran, "时间: 12:00");
  }

  #[test]
  fn parse_multiline() {
    let msg = "```text\n1: Hello\n中文: 你\n好\n```\n";
    let reply = prompt().parse(msg);
    assert!(!reply.bad);
    assert_eq!(&*reply.block_li[0].tran, "你好");
  }

  #[test]
  fn parse_bad() {
    let reply = prompt().parse("1: Hello\n");
    assert!(reply.bad);
    assert!(reply.block_li.is_empty());
  }

  #[test]
  fn fix_compact() {
    let src_li = ["a", "b", "c"];
    let txt = Txt::new(&src_li);
    let why_li = [None, Some(Why::Orig), None];
    let wrong_li = [None, Some("误".to_owned()), None];
    let got = prompt().fix(&txt.line_li, &[1, 2], &why_li, &wrong_li, true);
    assert!(got.contains("- 2:原文与输入不一致，原文：b，错误译文：误"));
    assert!(got.contains("2:b\n"));
    assert!(got.contains("3:c\n"));
  }
}
