use std::path::PathBuf;

use aitran_lang::{Code, by_code};
use bit_set::BitSet;
use serde::Deserialize;
use strum::VariantArray;

use crate::{Error, Result};

/// 配置根：任务列表
#[derive(Debug, Clone, Default)]
pub struct Conf {
  pub task_li: Vec<Task>,
}

/// 单个翻译任务
#[derive(Debug, Clone)]
pub struct Task {
  /// 待翻译目录（必填）
  pub dir: PathBuf,
  /// 源语言
  pub from: Code,
  /// 目标语言；配置为空时展开为全部语言
  pub to_li: Vec<Code>,
}

#[derive(Deserialize)]
#[serde(transparent)]
struct RawConf(Vec<RawTask>);

#[derive(Deserialize)]
struct RawTask {
  dir: Option<PathBuf>,
  from: Option<String>,
  #[serde(default)]
  to: ToRaw,
}

/// `to` 支持列表，也容忍空值
#[derive(Deserialize)]
#[serde(untagged)]
enum ToRaw {
  One(String),
  Many(Vec<String>),
}

impl Default for ToRaw {
  fn default() -> Self {
    Self::Many(Vec::new())
  }
}

impl ToRaw {
  fn code_li(self) -> Result<Vec<Code>> {
    match self {
      Self::Many(li) => li.into_iter().map(|text| lang(&text)).collect(),
      // `to:` 后没有内容时，NestedText 会给出空字符串
      Self::One(text) if text.trim().is_empty() => Ok(Vec::new()),
      Self::One(text) => lang(&text).map(|code| vec![code]),
    }
  }
}

pub(crate) fn parse(text: &str) -> Result<Option<Conf>> {
  let Some(raw) = crate::parse_as::<RawConf>(text)? else {
    return Ok(None);
  };
  let mut task_li = Vec::with_capacity(raw.0.len());
  for (i, raw) in raw.0.into_iter().enumerate() {
    let Some(dir) = raw.dir.filter(|dir| !dir.as_os_str().is_empty()) else {
      warn_skip(i, "dir");
      continue;
    };
    let Some(from) = raw.from.filter(|from| !from.trim().is_empty()) else {
      warn_skip(i, "from");
      continue;
    };
    let from = lang(&from)?;
    let mut to_li = raw.to.code_li()?;
    // 为空即全部语言（排除源语言）
    if to_li.is_empty() {
      to_li.extend(Code::VARIANTS.iter().copied().filter(|code| *code != from));
    }
    // 去重并排除源语言，避免重复翻译
    // 判别式取值范围固定，容量按语言总数给足，避免按列表长度反复扩容
    let mut seen = BitSet::with_capacity(Code::VARIANTS.len());
    to_li.retain(|&code| code != from && seen.insert(code as usize));
    task_li.push(Task { dir, from, to_li });
  }
  Ok(Some(Conf { task_li }))
}

fn lang(text: &str) -> Result<Code> {
  by_code(text).ok_or_else(|| Error::Lang(text.to_owned()))
}

/// 配置缺字段时跳过该任务并告警
fn warn_skip(index: usize, field: &str) {
  eprintln!("aitran_conf: 第 {} 个任务缺少 {field}，已忽略", index + 1);
}
