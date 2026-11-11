//! 行拆分、过滤与还原

// 与 Txt::restore 区分：拼接前缀与正文
use md_prefix::{Part, restore as join};

use crate::{fence::Fence, noise::only_noise};

/// 一行：原文 1 基行号、行首前缀与正文，内容均为原串切片
#[derive(Debug, Clone, Copy)]
pub struct Line<'a> {
  /// 在原文中的 1 基行号
  pub nth: usize,
  /// 行首前缀：空白与 Markdown 结构标记
  pub prefix: &'a str,
  /// 待译正文，已去行尾空白
  pub body: &'a str,
}

/// 整篇输入：待译行与直通行的原样信息，用于还原整篇
#[derive(Debug)]
pub struct Txt<'a> {
  /// 需要翻译的行，保持原文顺序
  pub line_li: Vec<Line<'a>>,
  /// 直通行（空行、代码块外的纯标点 / 表情行），保持原文顺序
  keep_li: Vec<Line<'a>>,
}

impl<'a> Txt<'a> {
  /// 单遍扫描整篇原始行
  ///
  /// 围栏代码块内的行只拆缩进，避免把 `- `、`## ` 等代码内容当 Markdown 标记；
  /// 代码块内的纯符号行仍需翻译，由提示词约束只翻注释。
  pub fn new<S: AsRef<str> + 'a>(src_li: impl IntoIterator<Item = &'a S>) -> Self {
    let src_li = src_li.into_iter();
    let mut fence = Fence::default();
    let mut line_li = Vec::with_capacity(src_li.size_hint().0);
    let mut keep_li = Vec::new();
    for (i, s) in src_li.enumerate() {
      let src = s.as_ref();
      let code = fence.step(src);
      let part = Part::new(src, !code);
      let line = Line {
        nth: i + 1,
        prefix: part.prefix,
        body: part.body,
      };
      if need_tran(&line, code) {
        line_li.push(line);
      } else {
        keep_li.push(line);
      }
    }
    Self { line_li, keep_li }
  }

  /// 待缓存原文正文：已去前缀与行尾空白，不含空行
  pub fn body_li(&self) -> impl ExactSizeIterator<Item = &str> + '_ {
    self.line_li.iter().map(|line| line.body)
  }

  /// 用译文按 `line_li` 顺序拼回整篇，直通行原样保留
  ///
  /// 译文少于待译行时，缺的行只留前缀。
  pub fn restore<I, B>(&self, tran_li: I) -> Vec<String>
  where
    I: IntoIterator<Item = B>,
    B: Into<String>,
  {
    let mut tran_li = tran_li.into_iter();
    let mut out = Vec::with_capacity(self.line_li.len() + self.keep_li.len());
    let mut ki = 0;
    for line in &self.line_li {
      // 先输出行号更小的直通行，保持原文顺序
      while let Some(keep) = self.keep_li.get(ki) {
        if keep.nth >= line.nth {
          break;
        }
        out.push(join(keep.prefix, keep.body));
        ki += 1;
      }
      let mut body = tran_li.next().map_or_else(String::new, Into::into);
      let start = body.len() - body.trim_start().len();
      if start > 0 {
        body.drain(..start);
      }
      body.truncate(body.trim_end().len());
      // 无前缀时直接复用译文，省一次分配
      out.push(if line.prefix.is_empty() {
        body
      } else {
        join(line.prefix, &body)
      });
    }
    // 末尾剩余的直通行
    for line in &self.keep_li[ki..] {
      out.push(join(line.prefix, line.body));
    }
    out
  }
}

/// 空正文行不送模型
fn blank(line: &Line<'_>) -> bool {
  line.body.is_empty()
}

/// 非空且代码块内或非纯符号
fn need_tran(line: &Line<'_>, code: bool) -> bool {
  !blank(line) && (code || !only_noise(line.body))
}
