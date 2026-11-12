//! 待译行按字符预算分批

use std::{iter::from_fn, ops::Range};

use aitran_parse::Line;

/// 单批原文的行字符上限，单行超限时独占一批
pub const CHAR_LIMIT: usize = 10_000;

/// 按字符预算切分待译行，返回行号区间
///
/// 区间保持原文顺序、互不重叠，合起来覆盖全部待译行；单行超限时独占一批。
pub(crate) fn range_li<'b, const LIMIT: usize>(
  line_li: &'b [Line<'_>],
) -> impl Iterator<Item = Range<usize>> + 'b {
  let mut pos = 0;
  from_fn(move || {
    if pos >= line_li.len() {
      return None;
    }
    let start = pos;
    let mut used = 0usize;
    while pos < line_li.len() {
      let body = line_li[pos].body;
      // ASCII 行字节数即字符数，省一次 UTF-8 解码；多字节行才逐字符计数
      let n = if body.is_ascii() {
        body.len()
      } else {
        body.chars().count()
      };
      let next = used.saturating_add(n);
      if pos > start && next > LIMIT {
        break;
      }
      used = next;
      pos += 1;
    }
    Some(start..pos)
  })
}

#[cfg(test)]
mod test {
  use std::ops::Range;

  use aitran_parse::Txt;

  use super::{CHAR_LIMIT, range_li};

  #[test]
  fn split_exact() {
    // 两行正好等于上限，合为一批
    let half = CHAR_LIMIT / 2;
    let a = "a".repeat(half);
    let b = "b".repeat(half);
    let src_li = [a.as_str(), "", b.as_str()];
    let txt = Txt::new(&src_li);
    let got_li: Vec<_> = range_li::<CHAR_LIMIT>(&txt.line_li).collect();
    assert_eq!(got_li.len(), 1);
    assert_eq!(got_li[0], 0..2);
  }

  #[test]
  fn split_over() {
    // 超限单行独占一批，后续行另起一批
    let long = "a".repeat(CHAR_LIMIT + 1);
    let src_li = [long.as_str(), "b"];
    let txt = Txt::new(&src_li);
    let got_li: Vec<_> = range_li::<CHAR_LIMIT>(&txt.line_li).collect();
    assert_eq!(got_li, [0..1, 1..2]);
  }

  #[test]
  fn cover_all() {
    let a = "a".repeat(CHAR_LIMIT / 2);
    let b = "b".repeat(CHAR_LIMIT / 2);
    let src_li = [a.as_str(), "", "...", "```", "});", "```", &b];
    let txt = Txt::new(&src_li);
    let mut end = 0;
    for Range { start, end: stop } in range_li::<CHAR_LIMIT>(&txt.line_li) {
      assert_eq!(start, end);
      let cost: usize = txt.line_li[start..stop]
        .iter()
        .map(|line| line.body.chars().count())
        .sum();
      assert!(cost <= CHAR_LIMIT || stop - start == 1);
      end = stop;
    }
    assert_eq!(end, txt.line_li.len());
  }

  #[test]
  fn split_small_limit() {
    let src_li = ["aaa", "bb", "c"];
    let txt = Txt::new(&src_li);
    let got_li: Vec<_> = range_li::<4>(&txt.line_li).collect();
    assert_eq!(got_li, [0..1, 1..3]);
  }

  #[test]
  fn split_small_multibyte() {
    let src_li = ["中中", "文文"];
    let txt = Txt::new(&src_li);
    let got_li: Vec<_> = range_li::<3>(&txt.line_li).collect();
    assert_eq!(got_li, [0..1, 1..2]);
  }
}
