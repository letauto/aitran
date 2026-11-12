use std::process::exit;

use ai_api::clean_text as clean;

#[compio::main]
async fn main() {
  if let Err(e) = aitran_log::init("aitran") {
    eprintln!("aitran: 日志初始化失败: {}", clean(&e.to_string()));
  }
  if let Err(e) = aitran::run().await {
    eprintln!("aitran: {}", clean(&e.to_string()));
    exit(1);
  }
}
