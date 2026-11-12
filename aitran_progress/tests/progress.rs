use std::time::Duration;

use aitran_progress::{Progress, eta_text};

#[test]
fn weighted_percent() {
  let mut p = Progress::new(1000, 3);
  assert_eq!(p.view().permille, 0);
  p.finish(250);
  assert_eq!(p.view().permille, 250);
  p.finish(750);
  assert_eq!(p.view().permille, 1000);
  assert_eq!(p.done_job(), 2);
}

#[test]
fn clamp_to_total() {
  let mut p = Progress::new(100, 1);
  p.finish(200);
  assert_eq!(p.view().permille, 1000);
  assert_eq!(p.view().eta, Some(Duration::ZERO));
}

#[test]
fn skip_reduces_base() {
  let mut p = Progress::new(100, 2);
  p.skip(40);
  assert_eq!(p.total_job(), 1);
  assert_eq!(p.view().permille, 0);
  p.finish(60);
  assert_eq!(p.view().permille, 1000);
  assert_eq!(p.done_job(), 1);
}

#[test]
fn skip_all_is_complete() {
  let mut p = Progress::new(100, 1);
  p.skip(100);
  assert_eq!(p.total_job(), 0);
  assert_eq!(p.view().permille, 1000);
  assert_eq!(p.view().eta, Some(Duration::ZERO));
}

#[test]
fn zero_weight_is_complete() {
  let p = Progress::new(0, 0);
  assert_eq!(p.view().permille, 1000);
  assert_eq!(p.view().eta, Some(Duration::ZERO));
}

#[test]
fn huge_weight_no_overflow() {
  let mut p = Progress::new(u64::MAX, 2);
  p.finish(1);
  assert_eq!(p.view().permille, 0);
  p.finish(u64::MAX);
  assert_eq!(p.view().permille, 1000);
  assert_eq!(p.view().eta, Some(Duration::ZERO));
}

#[test]
fn finish_without_start_no_eta() {
  // 未 start 时耗时恒为零，无法外推
  let mut p = Progress::new(100, 2);
  p.finish(50);
  assert_eq!(p.view().permille, 500);
  assert_eq!(p.view().eta, None);
}

#[test]
fn multi_round_elapsed() {
  let mut p = Progress::new(200, 2);
  p.start();
  p.finish(100);
  p.start();
  p.finish(100);
  assert_eq!(p.view().permille, 1000);
  assert_eq!(p.view().eta, Some(Duration::ZERO));
}

#[test]
fn eta_format() {
  assert_eq!(eta_text(None), "估算中");
  assert_eq!(eta_text(Some(Duration::ZERO)), "完成");
  assert_eq!(eta_text(Some(Duration::from_millis(1))), "1s");
  assert_eq!(eta_text(Some(Duration::from_secs(72))), "1m12s");
  assert_eq!(eta_text(Some(Duration::from_secs(3661))), "1h01m");
}

#[test]
fn eta_max_no_overflow() {
  // as_secs 为 u64::MAX 时进位不得溢出
  assert!(eta_text(Some(Duration::MAX)).contains('h'));
}
