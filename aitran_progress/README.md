[English](#en) | [中文](#zh)

---

<a name="en"></a>

# aitran_progress

Weighted progress tracking and remaining-time estimation based on completed work.

- [Install](#install)
- [Model](#model)
- [Usage](#usage)
- [API](#api)

## Install

```toml
[dependencies]
aitran_progress = "0.1"
```

## Model

- Each task carries a caller-defined weight, usually source file bytes
- Progress and ETA lookup are O(1)
- `start()` / `finish()` only accumulate time spent on real work
- `skip()` removes untimed tasks from the total

## Usage

```rust
use aitran_progress::{Progress, eta_text};

let mut progress = Progress::new(1000, 3);
progress.start();
progress.finish(250);

let view = progress.view();
assert_eq!(view.permille, 250);
assert_eq!(view.done_job, 1);
println!("{}", eta_text(view.eta));
```

## API

| Method | Description |
| ------ | ----------- |
| `Progress::new(total_weight, total_job)` | Create state |
| `start()` | Start timing the current task |
| `finish(weight)` | Complete a task and add its weight |
| `skip(weight)` | Skip a task and remove it from the total |
| `view()` | Snapshot of permille, ETA and job counts |
| `eta_text(Option<Duration>)` | Format as `11s`, `1m12s`, etc. |


---

<a name="zh"></a>

# aitran_progress

按权重计算整体进度，并用已完成任务的实际耗时估算剩余时间。

- [安装](#安装)
- [模型](#模型)
- [用法](#用法)
- [接口](#接口)

## 安装

```toml
[dependencies]
aitran_progress = "0.1"
```

## 模型

- 每个任务权重由调用方决定，通常使用源文件字节数
- 进度和 ETA 查询均为 O(1)
- `start()` / `finish()` 只累计实际工作的耗时
- `skip()` 从总数中扣除不计时任务

## 用法

```rust
use aitran_progress::{Progress, eta_text};

let mut progress = Progress::new(1000, 3);
progress.start();
progress.finish(250);

let view = progress.view();
assert_eq!(view.permille, 250);
assert_eq!(view.done_job, 1);
println!("{}", eta_text(view.eta));
```

## 接口

| 方法 | 说明 |
| ---- | ---- |
| `Progress::new(total_weight, total_job)` | 初始化 |
| `start()` | 开始当前任务计时 |
| `finish(weight)` | 完成任务并累计权重 |
| `skip(weight)` | 跳过任务，从总数扣除 |
| `view()` | 返回百分比、ETA、完成数快照 |
| `eta_text(Option<Duration>)` | 格式化为 `11s`、`1m12s` 等 |

