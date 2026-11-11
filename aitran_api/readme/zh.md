# aitran_api

逐行批量翻译：行预处理、提示词生成、分批、回复解析和多轮修订。

## 安装

```toml
[dependencies]
aitran_api = "0.1"
```

## 流程

1. `aitran_parse::Txt` 拆分前缀与正文，过滤空行和纯符号行
2. 按字符预算分批
3. 生成「行号: 原文」提示词
4. 解析模型回复中的 `行号 / 原文 / 译文` 块
5. 缺失、原文错误、空译文的行进入下一轮修订
6. 连续 3 轮没有进展才停止

## 用法

```rust
use aitran_api::tran_with;

let out = tran_with(
  &api_li,
  "zh",
  "en",
  &["你好", "世界"],
).await?;
assert_eq!(out, ["Hello", "World"]);
```

## 接口

| 接口 | 说明 |
| ---- | ---- |
| `tran_with` | 无回调翻译 |
| `tran_hook` | 带思考/正文流式回调 |
| `tran_ask_hook_cache` | 带行级缓存与提示词回调 |
| `tran_with_limit::<LIMIT>` | 指定单批字符上限，便于测试 |
| `CHAR_LIMIT` | 默认单批字符数 10_000 |
| `Hooks` | 思考、正文、提示词回调集合 |
| `OnPrompt` | 捕获系统提示和实际用户消息 |

## 修订

- 回复按单独一行 `-` 切块
- 每个块包含行号、原文、译文
- 原文必须与输入一致
- 译文逐行 trim，必须单行
- 待修订行数只减不增，连续 `STALL = 3` 轮不下降后返回 `Error::Format`
