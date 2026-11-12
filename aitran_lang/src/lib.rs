#![cfg_attr(docsrs, feature(doc_cfg))]

//! Google Translate 官方支持语言的代码、自称（endonym）与英文名。

mod by_code;
mod code;
mod name;

pub use by_code::{by_code, code_with_name_if_exist};
pub use code::Code;
pub use name::{EN, NAME, ZH};
