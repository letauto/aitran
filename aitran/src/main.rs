use std::process::exit;

#[compio::main]
async fn main() {
  if let Err(e) = aitran_log::init("aitran") {
    eprintln!("aitran: 日志初始化失败: {}", aitran::clean(&e.to_string()));
  }
  if let Err(e) = aitran::run().await {
    eprintln!("aitran: {}", aitran::clean(&e.to_string()));
    exit(1);
  }
}
