# aitran_db

纯文本、可 diff 的 i18n 数据库：只追加、流式压实、按 hash 找回旧原文。

## 安装

```toml
[dependencies]
aitran_db = "0.1"
```

## 存储格式

```text
@path/lang/<源文件 hash>
>line
>line
```

- `@` 开头是记录头，按最后两个 `/` 拆出 path、lang、hash
- `>` 开头是内容行
- 同一 `(path, lang)` 后写覆盖先写
- hash 为源文件内容的 BLAKE3 base64url，用于关联旧原文

## 用法

```rust
use aitran_db::Db;

let db = Db::open("i18n.db")?;
db.set("a.md", "zh", "hash", &["你好", "世界"]).await?;
let record = db.get_with_off("a.md", "zh").await?.unwrap();
assert_eq!(record.line_li, ["你好", "世界"]);

let old = db.get_hash("a.md", "zh", "hash").await?;
assert!(old.is_some());
```

## 接口

| 接口 | 说明 |
| ---- | ---- |
| `Db::open(path)` | 打开并获取进程文件锁 |
| `set(path, lang, hash, lines)` | 追加记录 |
| `get(path, lang)` | 读取最新内容 |
| `get_with_off(path, lang)` | 返回内容和起始偏移 |
| `get_hash(path, lang, hash)` | 按 hash 查找旧记录 |
| `get_hash_any(hash, lang)` | 跨路径按 hash 查找记录，用于识别源文件重命名 |
| `compact(paths)` | 只保留白名单路径并压实 |
| `compact_with_first(paths, lang)` | 源语言记录优先，丢弃失效译文 |

## 索引与复杂度

首次读取扫描一次文件，建立 `path -> lang -> offset` 和
`hash\0lang\0path -> offset` 索引。之后读取按偏移直读，读取目标记录
O(记录长度)，索引查询平均 O(1)。压实为流式扫描加临时文件替换。

`get_hash_any` 使用 `hash\0lang -> (path, offset)` 索引，源文件重命名后可
直接复用旧路径的译文缓存，再由调用方把结果追加到新路径。
