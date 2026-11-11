//! 底层逐行翻译
//!
//! 输入是一批待译行，行内容已预处理，不做过滤与 trim。
//!
//! 修订轮数不设上限：待修订行数只减不增，连续 STALL 轮不下降才放弃，
//! 因此每 3 轮至少少一行，最多 3n 轮收敛。

use ai_api::{Api, Chat, OnReply, OnThink};
use aitran_parse::Line;

use crate::{
  OnPrompt,
  error::{Error, Result},
  prompt::{Prompt, Why},
};

/// 连续多少轮待修订行数不下降就放弃
const STALL: usize = 3;

/// 翻译一批行，返回与批次等长、按批内顺序对齐的译文
pub(crate) async fn tran<T: OnThink, R: OnReply, P: OnPrompt>(
  api_li: &[Api],
  prompt: &Prompt,
  line_li: &[Line<'_>],
  on_think: &mut T,
  on_reply: &mut R,
  on_prompt: &mut P,
) -> Result<Vec<String>> {
  let n = line_li.len();
  if n == 0 {
    return Ok(Vec::new());
  }
  let mut out_li = vec![String::new(); n];
  let mut done = vec![false; n];
  let mut why_li = vec![None::<Why>; n];
  // 上一轮的错误译文，与批内序号对齐
  let mut wrong_li = vec![None::<String>; n];
  let mut pending_li: Vec<usize> = (0..n).collect();

  let sys = prompt.system();
  on_prompt.system(&sys);
  let mut chat = Chat::new(api_li, &mut *on_think, &mut *on_reply);
  let mut session = chat.session(sys);
  let mut bad = false;

  let mut prev = n;
  let mut stall = 0;
  let mut first = true;
  loop {
    let msg = if first {
      Prompt::ask(line_li)
    } else {
      prompt.fix(line_li, &pending_li, &why_li, &wrong_li, bad)
    };
    first = false;
    on_prompt.ask(&msg);
    let got = session.ask(msg).await?;
    let reply = prompt.parse(&got.text);
    bad = reply.bad;
    // 只清理本轮待修订行的旧状态，避免每轮扫全部行
    for &k in &pending_li {
      why_li[k] = None;
      wrong_li[k] = None;
    }

    for b in reply.block_li {
      let Some(k) = b.nth.checked_sub(1).filter(|&k| k < n) else {
        bad = true;
        continue;
      };
      // 已译好的行忽略重复块
      if done[k] {
        continue;
      }
      // 原文比对去首尾空白；提示词里仍用原文
      let orig = line_li[k].body;
      if b.src != orig {
        why_li[k] = Some(Why::Orig);
        // 非空才值得回给模型看
        if !b.tran.is_empty() {
          wrong_li[k] = Some(b.tran.into_owned());
        }
        continue;
      }
      // 校验通过才取译文所有权，避免错误块多做一次分配
      let tran = b.tran.into_owned();
      if tran.is_empty() && !orig.is_empty() {
        why_li[k] = Some(Why::Empty);
        continue;
      }
      out_li[k] = tran;
      done[k] = true;
    }

    pending_li.retain(|&k| !done[k]);
    if pending_li.is_empty() {
      return Ok(out_li);
    }
    // 待修订行数只减不增，连续 STALL 轮不降说明模型已无法推进
    if pending_li.len() < prev {
      prev = pending_li.len();
      stall = 0;
    } else {
      stall += 1;
      if stall >= STALL {
        break;
      }
    }
  }

  Err(Error::Format(
    pending_li.iter().map(|&k| line_li[k].nth).collect(),
  ))
}
