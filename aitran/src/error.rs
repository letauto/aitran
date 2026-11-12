use std::{io, path::PathBuf, result};

use thiserror::Error;

#[derive(Error, Debug)]
#[non_exhaustive]
pub enum Error {
  #[error(transparent)]
  Io(#[from] io::Error),
  #[error(transparent)]
  FindConf(#[from] find_conf::Error),
  #[error(transparent)]
  Conf(#[from] aitran_conf::Error),
  #[error(transparent)]
  Api(#[from] aitran_api::Error),
  #[error(transparent)]
  Db(#[from] aitran_db::Error),
  #[error(transparent)]
  FindExt(#[from] find_ext::Error),
  /// 找不到任务配置
  #[error("找不到 aitran.nt")]
  NoConf,
  /// 显式指定的配置文件不存在
  #[error("配置文件不存在: {0}")]
  ConfNotFound(PathBuf),
  /// 配置里没有任务
  #[error("任务配置为空")]
  NoTask,
  /// 没有可用的 API 端点
  #[error("没有可用的 API 端点")]
  NoApi,
}

pub type Result<T> = result::Result<T, Error>;
