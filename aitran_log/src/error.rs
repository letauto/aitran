use std::result;

/// crate 统一返回类型
pub type Result<T, E = Error> = result::Result<T, E>;

/// aitran_log 错误
#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error(transparent)]
  Logforth(#[from] logforth::Error),
}
