# aitran_progress

按权重计算整体进度，并用已完成任务的实际耗时估算剩余时间。

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
