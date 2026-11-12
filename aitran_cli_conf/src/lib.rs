#![cfg_attr(docsrs, feature(doc_cfg))]

use std::{
  env,
  path::{Path, PathBuf},
  sync::OnceLock,
};

mod conf;

use aitran_conf::Result;
pub use conf::Conf;

/// 直接指定配置文件路径的环境变量
const CONF_PATH: &str = "AII18N_CONF_PATH";
/// 配置文件名
const FILE: &str = "aii18n.nt";

fn resolve() -> PathBuf {
  match env::var_os(CONF_PATH) {
    Some(path) if !path.is_empty() => PathBuf::from(path),
    _ => confdir::confdir().join(FILE),
  }
}

/// 配置文件路径，`AII18N_CONF_PATH` 优先于系统配置目录
pub fn conf() -> &'static Path {
  static CONF: OnceLock<PathBuf> = OnceLock::new();
  CONF.get_or_init(resolve)
}

/// 读取并解析配置文件，文件不存在或为空时返回 `Ok(None)`
pub fn load() -> Result<Option<Conf>> {
  aitran_conf::load_as(conf())
}
