//! Markdown 围栏代码块相关用例

mod common;

use aitran_api::tran_with;
use aok::{OK, Void};
use common::{api, serve_rec, sse};

/// 围栏代码块内的纯符号行不直通，围栏行也送模型，提示词约束只翻注释
#[compio::test]
async fn tran_code_block() -> Void {
  let (addr, seen) = serve_rec(|_, _| {
    sse(
      "\
1: ```js
中文: ```js
-
2: });
中文: });
-
3: // 计数
中文: // count
-
4: ```
中文: ```
",
    )
  });
  let api_li = [api(addr)];
  let src_li = ["```js", "});", "// 计数", "```"];
  let out = tran_with(&api_li, "English", "中文", &src_li).await?;
  assert_eq!(out, ["```js", "});", "// count", "```"]);
  let req = seen.lock().unwrap();
  assert_eq!(req.len(), 1);
  // 纯符号行与围栏行都进了请求
  assert!(req[0].contains("2:});"), "{}", req[0]);
  assert!(req[0].contains("1:```"), "{}", req[0]);
  // 系统提示包含代码块规则
  assert!(req[0].contains("只翻译注释"), "{}", req[0]);
  OK
}

/// 不同围栏字符、更长围栏不提前闭合，块外纯符号行仍直通
#[compio::test]
async fn tran_fence_nested() -> Void {
  let (addr, seen) = serve_rec(|_, _| {
    sse(
      "\
1: ```
中文: ```
-
2: ~~~
中文: ~~~
-
3: });
中文: });
-
4: ~~~~
中文: ~~~~
-
5: ````
中文: ````
",
    )
  });
  let api_li = [api(addr)];
  let src_li = ["```", "~~~", "});", "~~~~", "````", "..."];
  let out = tran_with(&api_li, "English", "中文", &src_li).await?;
  assert_eq!(out, src_li);
  let req = seen.lock().unwrap();
  assert_eq!(req.len(), 1);
  // 只有更长的同字符围栏能闭合，闭合后回到直通
  assert!(req[0].contains(r"2:~~~\n"), "{}", req[0]);
  assert!(req[0].contains(r"4:~~~~\n"), "{}", req[0]);
  assert!(!req[0].contains("6:..."), "{}", req[0]);
  OK
}

/// 带信息串的围栏不闭合；未闭合围栏延续到文末
#[compio::test]
async fn tran_fence_unclosed() -> Void {
  let (addr, seen) = serve_rec(|_, _| {
    sse(
      "\
1: ~~~py
中文: ~~~py
-
2: ~~~text
中文: ~~~text
-
3: | --- |
中文: | --- |
-
4: ```
中文: ```
-
5: ...
中文: ...
",
    )
  });
  let api_li = [api(addr)];
  let src_li = ["~~~py", "~~~text", "| --- |", "```", "..."];
  let out = tran_with(&api_li, "English", "中文", &src_li).await?;
  assert_eq!(out, src_li);
  let req = seen.lock().unwrap();
  assert_eq!(req.len(), 1);
  // 信息串挡住闭合，异字符围栏也不闭合，其后纯符号行仍送模型
  assert!(req[0].contains("3:| --- |"), "{}", req[0]);
  assert!(req[0].contains("5:..."), "{}", req[0]);
  OK
}

/// 围栏最多缩进 3 个空格；4 空格缩进的 ``` 是块内内容，不能提前闭合
#[compio::test]
async fn tran_fence_indent() -> Void {
  let (addr, seen) = serve_rec(|_, _| {
    sse(
      "\
1: ```js
中文: ```js
-
2: ```
中文: ```
-
3: });
中文: });
-
4: ```
中文: ```
",
    )
  });
  let api_li = [api(addr)];
  let src_li = ["   ```js", "    ```", "});", "   ```", "..."];
  let out = tran_with(&api_li, "English", "中文", &src_li).await?;
  assert_eq!(out, src_li);
  let req = seen.lock().unwrap();
  assert_eq!(req.len(), 1);
  // 4 空格行没有闭合围栏，其后的纯符号行仍送模型
  assert!(req[0].contains("3:});"), "{}", req[0]);
  // 3 空格围栏闭合后，块外纯符号行恢复直通
  assert!(!req[0].contains("5:..."), "{}", req[0]);
  OK
}

/// 顶层 4 空格缩进属于缩进代码，不算围栏，其后的纯符号行照常直通
#[compio::test]
async fn tran_fence_indent_top() -> Void {
  let (addr, seen) = serve_rec(|_, _| sse(""));
  let api_li = [api(addr)];
  let src_li = ["    ```", "..."];
  let out = tran_with(&api_li, "English", "中文", &src_li).await?;
  assert_eq!(out, src_li);
  assert!(seen.lock().unwrap().is_empty());
  OK
}

/// 反引号围栏的信息串不能含反引号：```a``` 是行内代码，不开启代码块
#[compio::test]
async fn tran_fence_inline() -> Void {
  let (addr, seen) = serve_rec(|_, _| sse("1: ```a```\n中文: 甲\n"));
  let api_li = [api(addr)];
  let src_li = ["```a```", "..."];
  let out = tran_with(&api_li, "English", "中文", &src_li).await?;
  assert_eq!(out, ["甲", "..."]);
  let req = seen.lock().unwrap();
  assert_eq!(req.len(), 1);
  assert!(!req[0].contains("2:..."), "{}", req[0]);
  OK
}

/// 关闭围栏允许行尾空白
#[compio::test]
async fn tran_fence_close_ws() -> Void {
  let (addr, seen) = serve_rec(|_, _| {
    sse(
      "\
1: ```
中文: ```
-
2: });
中文: });
-
3: ```
中文: ```
",
    )
  });
  let api_li = [api(addr)];
  let src_li = ["```", "});", "```   ", "..."];
  let out = tran_with(&api_li, "English", "中文", &src_li).await?;
  assert_eq!(out, ["```", "});", "```", "..."]);
  let req = seen.lock().unwrap();
  assert_eq!(req.len(), 1);
  assert!(!req[0].contains("4:..."), "{}", req[0]);
  OK
}
