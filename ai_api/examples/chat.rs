//! 用环境变量做多轮流式对话
//!
//! cargo run -p ai_api --example chat

use std::{
  env,
  io::{self, IsTerminal, Write},
};

use ai_api::{Api, Chat, Flow, OnReply, OnThink, Think, fill_line};

/// 灰色（亮黑）
const GRAY: &str = "\x1b[90m";
const RESET: &str = "\x1b[0m";

/// 思考：灰色输出
struct Gray {
  /// 终端时才着色
  tty: bool,
  /// 是否已进入灰色（`end` 复位）
  on: bool,
}

impl OnThink for Gray {
  fn think(&mut self, s: &str) -> Flow {
    if !self.on {
      if self.tty {
        print!("{GRAY}");
      }
      self.on = true;
    }
    print!("{s}");
    let _ = io::stdout().flush();
    Flow::Continue
  }

  fn end(&mut self, think: &str) {
    if !self.on {
      return;
    }
    if self.tty {
      print!("{RESET}");
    }
    self.on = false;
    fill_line(think);
    let _ = io::stdout().flush();
  }
}

/// 正文：直接打印，结束时补换行
struct Out;

impl OnReply for Out {
  fn reply(&mut self, s: &str) -> Flow {
    print!("{s}");
    let _ = io::stdout().flush();
    Flow::Continue
  }

  fn end(&mut self, text: &str) {
    fill_line(text);
    let _ = io::stdout().flush();
  }
}

#[compio::main]
async fn main() -> aok::Void {
  log_init::init();
  let base = env::var("AI_API_BASE").map_err(|e| aok::anyhow!("AI_API_BASE: {e}"))?;
  let token = env::var("AI_API_TOKEN").map_err(|e| aok::anyhow!("AI_API_TOKEN: {e}"))?;
  let model = env::var("AI_API_MODEL").map_err(|e| aok::anyhow!("AI_API_MODEL: {e}"))?;
  let api_li = [Api {
    base,
    token_li: vec![token],
    model,
    think: Think::default(),
  }];
  // 第二问考记忆：能否说出上一问
  let prompt_li = ["1 + 1 等于几？", "我上一个问题是什么？"];

  let tty = io::stdout().is_terminal();
  let mut think = Gray { tty, on: false };
  let mut reply = Out;
  let mut chat = Chat::new(&api_li, &mut think, &mut reply);
  let mut session = chat.session("回答要简洁");

  for prompt in prompt_li {
    println!("→ {prompt}");
    let got = session.ask(prompt).await?;
    eprintln!("{got:#?}");
  }
  Ok(())
}
