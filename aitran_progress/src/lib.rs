//! 加权进度与剩余时间估算
//!
//! 每个任务的权重是它要处理的源文件字节数；完成一个任务就把权重计入
//! `done`。进度与 ETA 都是 O(1) 查询，不保存每个文件的明细。

use std::{fmt::Write as _, time::Duration};

use coarsetime::Instant;

/// 加权进度状态
#[derive(Debug)]
pub struct Progress {
  total_weight: u64,
  done_weight: u64,
  total_job: usize,
  done_job: usize,
  begin: Option<Instant>,
  elapsed: Duration,
}

/// 某一时刻的只读快照
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct View {
  /// 千分比，1000 表示 100%
  pub permille: u16,
  /// 剩余时间；尚未完成任何任务时为 `None`
  pub eta: Option<Duration>,
  pub done_job: usize,
  pub total_job: usize,
}

impl Progress {
  /// `total_weight` 为全部任务权重之和，`total_job` 为任务数
  pub fn new(total_weight: u64, total_job: usize) -> Self {
    Self {
      total_weight,
      done_weight: 0,
      total_job,
      done_job: 0,
      begin: None,
      elapsed: Duration::ZERO,
    }
  }

  /// 开始计时；重复调用无副作用
  pub fn start(&mut self) {
    self.begin.get_or_insert_with(Instant::now);
  }

  /// 跳过不计时的任务：从总数中扣除，不增加完成数
  pub fn skip(&mut self, weight: u64) {
    self.total_weight = self.total_weight.saturating_sub(weight);
    self.total_job = self.total_job.saturating_sub(1);
    self.done_weight = self.done_weight.min(self.total_weight);
    self.done_job = self.done_job.min(self.total_job);
  }

  /// 完成一个任务
  pub fn finish(&mut self, weight: u64) {
    if let Some(begin) = self.begin.take() {
      self.elapsed = self.elapsed.saturating_add(begin.elapsed().into());
    }
    self.done_weight = self
      .done_weight
      .saturating_add(weight)
      .min(self.total_weight);
    self.done_job = self.done_job.saturating_add(1).min(self.total_job);
  }

  pub fn done_job(&self) -> usize {
    self.done_job
  }

  pub fn total_job(&self) -> usize {
    self.total_job
  }

  /// 当前进度与 ETA
  pub fn view(&self) -> View {
    View {
      permille: self.permille(),
      eta: self.eta(),
      done_job: self.done_job,
      total_job: self.total_job,
    }
  }

  /// 千分比，整数运算避免浮点误差与溢出
  fn permille(&self) -> u16 {
    if self.total_weight == 0 {
      return 1000;
    }
    let n = u128::from(self.done_weight).saturating_mul(1000) / u128::from(self.total_weight);
    n.min(1000) as u16
  }

  /// 按「已完成字节 / 已耗时」线性外推剩余时间
  ///
  /// 无已完成任务或耗时为零时返回 `None`；全部完成返回零时长。
  fn eta(&self) -> Option<Duration> {
    let remaining = self.total_weight.checked_sub(self.done_weight)?;
    if remaining == 0 {
      return Some(Duration::ZERO);
    }
    if self.done_weight == 0 {
      return None;
    }
    let elapsed = self.elapsed.as_nanos();
    if elapsed == 0 {
      return None;
    }
    let ns = u128::from(remaining).saturating_mul(elapsed) / u128::from(self.done_weight);
    let ns = u64::try_from(ns).unwrap_or(u64::MAX);
    Some(Duration::from_nanos(ns))
  }
}

/// 人类可读的剩余时间，向上取整到秒
pub fn eta_text(eta: Option<Duration>) -> String {
  let Some(eta) = eta else {
    return "估算中".to_owned();
  };
  if eta.is_zero() {
    return "完成".to_owned();
  }
  let secs = eta
    .as_secs()
    .saturating_add(u64::from(eta.subsec_nanos() > 0));
  // 时长上界 1.8e10 秒：小时数最多 7 位，加 h、两位分钟与 m 共 11 字符
  let mut out = String::with_capacity(11);
  if secs >= 3600 {
    let _ = write!(out, "{}h{:02}m", secs / 3600, secs % 3600 / 60);
  } else if secs >= 60 {
    let _ = write!(out, "{}m{:02}s", secs / 60, secs % 60);
  } else {
    let _ = write!(out, "{secs}s");
  }
  out
}
