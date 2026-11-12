#![cfg_attr(docsrs, feature(doc_cfg))]

//! 批量逐行翻译。
//!
//! 上游先用 [`aitran_parse::Txt`] 做行预处理：去掉行尾空白，拆出行首前缀
//! （空白 + 标题、引用、列表等 Markdown 标记），只把正文交给模型，译文写回时
//! 还原前缀；空行与纯标点/表情行直通，不进入模型。Markdown 围栏代码块
//! （含围栏行本身）除外：`});`、`| --- |` 这类纯符号行也要交给模型，由提示词
//! 约束只翻译注释。待译行按 [`CHAR_LIMIT`] 逐行累计分批，装满即调用底层逐行
//! 翻译，每批独立会话，避免上下文过长。
//!
//! 提示词生成、回复解析与分批都在本 crate，模型调用与多轮修订编排在 `raw`。
mod batch;
mod error;
mod prompt;
mod raw;

use std::ops::Range;

use ai_api::{Api, OnReply, OnThink};
use aitran_parse::{Line, Txt};
pub use batch::CHAR_LIMIT;
pub use error::{Error, Result};
use prompt::Prompt;

/// 请求提示词回调，便于调试实际发送内容
pub trait OnPrompt {
  /// 会话系统提示，每批建立会话时调用一次
  fn system(&mut self, _text: &str) {}
  /// 实际发送的用户消息，含修订轮
  fn ask(&mut self, text: &str);
}

/// 不需要提示词时传 `&mut ()`
impl OnPrompt for () {
  fn ask(&mut self, _text: &str) {}
}

/// 流式回调集合，避免长参数列表
pub struct HookSet<'a, T: OnThink, R: OnReply, P: OnPrompt> {
  pub think: &'a mut T,
  pub reply: &'a mut R,
  pub prompt: &'a mut P,
}

/// 指定端点逐行翻译
///
/// 返回与 `line_li` 等长、按下标对齐的译文。
pub async fn tran_with(
  api_li: &[Api],
  from_lang: impl AsRef<str>,
  to_lang: impl AsRef<str>,
  line_li: &[impl AsRef<str>],
) -> Result<Vec<String>> {
  tran_ask_hook(
    api_li,
    from_lang,
    to_lang,
    line_li,
    &mut (),
    &mut (),
    &mut (),
  )
  .await
}

/// 指定单批字符上限的逐行翻译，便于小批量测试
pub async fn tran_with_limit<const LIMIT: usize>(
  api_li: &[Api],
  from_lang: impl AsRef<str>,
  to_lang: impl AsRef<str>,
  line_li: &[impl AsRef<str>],
) -> Result<Vec<String>> {
  let txt = Txt::new(line_li);
  core::<LIMIT>(
    api_li,
    from_lang.as_ref(),
    to_lang.as_ref(),
    line_li,
    &txt,
    |_| None,
    HookSet {
      think: &mut (),
      reply: &mut (),
      prompt: &mut (),
    },
  )
  .await
}

/// 带行级缓存的逐行翻译
///
/// `cache` 收到待译行正文，返回已有译文时跳过该行，不再请求模型。
/// 空白缓存视为未命中。
/// 返回整篇译文，行数与输入一致，直通行原样保留。
pub async fn tran_ask_hook_cache<
  T: OnThink,
  R: OnReply,
  P: OnPrompt,
  C: FnMut(&str) -> Option<String>,
>(
  api_li: &[Api],
  from_lang: impl AsRef<str>,
  to_lang: impl AsRef<str>,
  line_li: &[impl AsRef<str>],
  txt: &Txt<'_>,
  cache: C,
  hook_set: HookSet<'_, T, R, P>,
) -> Result<Vec<String>> {
  core::<CHAR_LIMIT>(
    api_li,
    from_lang.as_ref(),
    to_lang.as_ref(),
    line_li,
    txt,
    cache,
    hook_set,
  )
  .await
}

/// 带提示词回调的逐行翻译
///
/// `on_think` / `on_reply` 收到模型的思考与正文增量，`on_prompt` 额外收到
/// 每批的系统提示与每次实际发送的用户消息。
pub async fn tran_ask_hook<T: OnThink, R: OnReply, P: OnPrompt>(
  api_li: &[Api],
  from_lang: impl AsRef<str>,
  to_lang: impl AsRef<str>,
  line_li: &[impl AsRef<str>],
  on_think: &mut T,
  on_reply: &mut R,
  on_prompt: &mut P,
) -> Result<Vec<String>> {
  let txt = Txt::new(line_li);
  core::<CHAR_LIMIT>(
    api_li,
    from_lang.as_ref(),
    to_lang.as_ref(),
    line_li,
    &txt,
    |_| None,
    HookSet {
      think: on_think,
      reply: on_reply,
      prompt: on_prompt,
    },
  )
  .await
}

/// 翻译核心：缓存过滤、直通与分批编排，`LIMIT` 为单批字符上限
///
/// 缓存恒未命中时即为无缓存路径。
async fn core<const LIMIT: usize>(
  api_li: &[Api],
  from: &str,
  to: &str,
  line_li: &[impl AsRef<str>],
  txt: &Txt<'_>,
  mut cache: impl FnMut(&str) -> Option<String>,
  hook_set: HookSet<'_, impl OnThink, impl OnReply, impl OnPrompt>,
) -> Result<Vec<String>> {
  if let Some(out) = passthrough(from, to, line_li) {
    return Ok(out);
  }
  let mut tran_li: Vec<String> = vec![String::new(); txt.line_li.len()];
  // 只把未命中的行交给模型，下标映射回原始待译行
  let mut miss_idx_li = Vec::new();
  let mut miss_line_li = Vec::new();
  for (k, line) in txt.line_li.iter().enumerate() {
    match cache(line.body) {
      Some(tran) if !tran.trim().is_empty() => tran_li[k] = tran,
      _ => {
        miss_idx_li.push(k);
        miss_line_li.push(*line);
      }
    }
  }
  if !miss_line_li.is_empty() {
    let HookSet {
      think: on_think,
      reply: on_reply,
      prompt: on_prompt,
    } = hook_set;
    let mut job = Job {
      api_li,
      prompt: Prompt::new(from, to),
      on_think,
      on_reply,
      on_prompt,
    };
    job
      .run::<LIMIT>(&mut tran_li, &miss_line_li, miss_idx_li.iter().copied())
      .await?;
  }
  Ok(txt.restore(tran_li))
}

/// 无需模型的直通情形：空输入或源语言等于目标语言
///
/// 同语言时两个标签同名会让回复解析产生歧义，因此不进入模型，
/// 只去掉每行行尾空白后原样返回。
fn passthrough(from: &str, to: &str, line_li: &[impl AsRef<str>]) -> Option<Vec<String>> {
  if line_li.is_empty() {
    return Some(Vec::new());
  }
  from.eq_ignore_ascii_case(to).then(|| {
    line_li
      .iter()
      .map(|line| line.as_ref().trim_end().to_owned())
      .collect()
  })
}

/// 一次批量翻译的上下文
struct Job<'a, T: OnThink, R: OnReply, P: OnPrompt> {
  api_li: &'a [Api],
  prompt: Prompt,
  on_think: &'a mut T,
  on_reply: &'a mut R,
  on_prompt: &'a mut P,
}

impl<T: OnThink, R: OnReply, P: OnPrompt> Job<'_, T, R, P> {
  /// 分批翻译，按 `slot_li` 把译文写回 `out`
  async fn run<const LIMIT: usize>(
    &mut self,
    out: &mut [String],
    line_li: &[Line<'_>],
    mut slot_li: impl Iterator<Item = usize>,
  ) -> Result<()> {
    // 批次与 slot_li 均按原文顺序连续对齐，`fetch` 返回与批次等长的译文
    for Range { start, end } in batch::range_li::<LIMIT>(line_li) {
      let n = end - start;
      let got_li = self.fetch(&line_li[start..end]).await?;
      for (slot, text) in slot_li.by_ref().take(n).zip(got_li) {
        out[slot] = text;
      }
    }
    Ok(())
  }

  /// 翻译一批行，返回与批次等长的译文
  async fn fetch(&mut self, line_li: &[Line<'_>]) -> Result<Vec<String>> {
    raw::tran(
      self.api_li,
      &self.prompt,
      line_li,
      &mut *self.on_think,
      &mut *self.on_reply,
      &mut *self.on_prompt,
    )
    .await
  }
}
