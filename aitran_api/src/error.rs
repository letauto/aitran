use std::result;

use thiserror::Error;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
  #[error(transparent)]
  Api(#[from] ai_api::Error),
  /// 多轮修订后仍有行未完成，值为 1 基行号
  #[error("翻译未完成: {0:?}")]
  Format(Vec<usize>),
}

pub type Result<T> = result::Result<T, Error>;
