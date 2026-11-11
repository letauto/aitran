# aitran_progress

Weighted progress tracking and remaining-time estimation based on completed work.

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
