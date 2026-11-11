mod en;
mod name_li;
mod zh;

pub use en::EN;
pub use name_li::NAME;
pub use zh::ZH;

/// 三张表都按 `Code` 判别式索引，长度必须与变体数一致；
/// unit 枚举无显式判别式，从 0 连续递增由语言保证
const _: () = {
  let n = <crate::Code as strum::VariantArray>::VARIANTS.len();
  assert!(EN.len() == n, "EN 长度需与 Code 变体数一致");
  assert!(NAME.len() == n, "NAME 长度需与 Code 变体数一致");
  assert!(ZH.len() == n, "ZH 长度需与 Code 变体数一致");
};
