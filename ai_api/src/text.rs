use std::borrow::Cow;

/// 终端安全文本：保留换行与制表符，去掉其他控制符和双向控制符
pub fn clean_text(text: &str) -> Cow<'_, str> {
  if text.chars().all(keep) {
    Cow::Borrowed(text)
  } else {
    Cow::Owned(text.chars().filter(|&c| keep(c)).collect())
  }
}

fn keep(c: char) -> bool {
  matches!(c, '\n' | '\t')
    || (!c.is_control()
      && !matches!(
        c,
        '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}'
      ))
}

#[cfg(test)]
mod test {
  use super::clean_text;

  #[test]
  fn strip() {
    assert_eq!(clean_text("a\x1b[31m\nb"), "a[31m\nb");
    assert_eq!(clean_text("a\u{202e}b\r"), "ab");
    assert_eq!(clean_text("普通\t文本\n"), "普通\t文本\n");
  }
}
