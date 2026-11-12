use std::{env, fs};

use ai_api::Think;
use aitran_cli_conf::Conf;
use aitran_conf::Error;
use aok::{OK, Void};

#[ctor::ctor(unsafe)]
fn _log_init() {
  log_init::init();
}

const TEXT: &str = concat!(
  "# 注释\n",
  "api:\n",
  "  -\n",
  "    base: https://api.example.com/v1\n",
  "    token:\n",
  "      - sk-1\n",
  "      - sk-2\n",
  "    model: deepseek-v4.1-flash\n",
  "    think: max\n",
  "  -\n",
  "    base: https://ollama.com/v1\n",
  "    token:\n",
  "      - sk-3\n",
  "    model: llama3\n",
);

#[test]
fn test_load_from() -> Void {
  let path = env::temp_dir().join("aii18n_conf_load_test.nt");
  fs::write(&path, TEXT)?;
  let conf = aitran_conf::load_as::<Conf>(&path)?;
  fs::remove_file(&path)?;
  let conf = conf.unwrap();
  assert_eq!(conf.api_li.len(), 2);
  let api = &conf.api_li[0];
  assert_eq!(api.base, "https://api.example.com/v1");
  assert_eq!(api.token_li, ["sk-1", "sk-2"]);
  assert_eq!(api.think, Think::Max);
  // 未配置时默认 medium
  assert_eq!(conf.api_li[1].think, Think::Medium);
  OK
}

#[test]
fn test_load_from_missing() -> Void {
  let path = env::temp_dir().join("aii18n_conf_missing_test.nt");
  let _ = fs::remove_file(&path);
  assert!(aitran_conf::load_as::<Conf>(&path)?.is_none());
  OK
}

/// env 路径与 `load` 走同一全局缓存，须在一个测试内顺序验证，避免并行竞争
#[test]
fn test_conf_env_load() -> Void {
  let path = env::temp_dir().join("aii18n_conf_env_test.nt");
  fs::write(&path, TEXT)?;
  unsafe { env::set_var("AII18N_CONF_PATH", &path) };
  assert_eq!(aitran_cli_conf::conf(), path.as_path());
  let conf = aitran_cli_conf::load()?.unwrap();
  fs::remove_file(&path)?;
  unsafe { env::remove_var("AII18N_CONF_PATH") };
  assert_eq!(conf.api_li[1].base, "https://ollama.com/v1");
  OK
}

#[test]
fn test_parse_err() -> Void {
  let path = env::temp_dir().join("aii18n_conf_err_test.nt");
  fs::write(&path, "key: a\nkey: b\n")?;
  let err = aitran_conf::load_as::<Conf>(&path).unwrap_err();
  fs::remove_file(&path)?;
  assert!(matches!(err, Error::Parse(_)));
  OK
}
