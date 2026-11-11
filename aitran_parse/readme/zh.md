# aitran_parse

翻译文本的行拆分、过滤与还原，与模型调用解耦。

## 安装

```toml
[dependencies]
aitran_parse = "0.1"
```

## 处理流程

输入原始 Markdown 行：

1. 拆分行首前缀与正文
2. 空行、代码块外的纯标点/表情行直通
3. 只把待译正文交给模型
4. 按原文行号还原前缀与直通行

围栏代码块内的纯符号行也会翻译，由提示词约束只翻注释。

## 用法

```rust
use aitran_parse::Txt;

let src_li = ["  ## Title", "```js", "});", "```", "..."];
let txt = Txt::new(&src_li);

assert_eq!(txt.line_li[0].prefix, "  ## ");
assert_eq!(txt.line_li[0].body, "Title");
assert_eq!(
  txt.restore(["标题", "```js", "});", "```"]),
  ["  ## 标题", "```js", "});", "```", "..."]
);
```

## 接口

| 接口 | 说明 |
| ---- | ---- |
| `Txt::new(&[&str])` | 单遍拆分并分类 |
| `Txt::line_li` | 待译行，保持原文顺序 |
| `Txt::body_li()` | 待缓存正文，已 trim、不含空行 |
| `Txt::restore(译文)` | 拼回整篇，译文逐条 trim |
| `Line { nth, prefix, body }` | 1 基行号、前缀、正文 |
