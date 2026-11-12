#![cfg_attr(docsrs, feature(doc_cfg))]

//! 翻译文本的行拆分、过滤与还原。
//!
//! 与模型调用完全解耦：输入原始行，[`Txt::line_li`] 只含需要翻译的行
//! （空行与代码块外的纯标点 / 表情行直通），译文按同样顺序交给
//! [`Txt::restore`] 拼回整篇，行号与原文一一对应。
//!
//! ```
//! use aitran_parse::Txt;
//!
//! let src_li = ["  ## Title", "```js", "});", "```", "..."];
//! let txt = Txt::new(&src_li);
//! assert_eq!(txt.line_li[0].prefix, "  ## ");
//! assert_eq!(txt.line_li[0].body, "Title");
//! // 围栏内纯符号行也要翻译，围栏行本身也送模型
//! assert_eq!(txt.line_li[2].body, "});");
//! // 围栏外的纯标点直通，不占译文位
//! assert_eq!(
//!   txt.restore(["标题", "```js", "});", "```"]),
//!   ["  ## 标题", "```js", "});", "```", "..."]
//! );
//! ```

mod fence;
mod line;
mod noise;

pub use line::{Line, Txt};
