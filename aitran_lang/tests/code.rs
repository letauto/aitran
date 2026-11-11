use aitran_lang::{Code, NAME, by_code, code_with_name_if_exist};
use strum::VariantArray as _;

#[test]
fn every_code_round_trips() {
  for &code in Code::VARIANTS {
    let s = code.as_str();
    assert_eq!(by_code(s), Some(code), "{s}");
    assert_eq!(by_code(s.to_ascii_uppercase()), Some(code), "{s}");
  }
}

#[test]
fn region_and_case() {
  assert_eq!(by_code("zh-CN"), Some(Code::Zh));
  assert_eq!(by_code("zh-TW"), Some(Code::ZhTW));
  assert_eq!(by_code("zh_HK"), Some(Code::ZhTW));
  assert_eq!(by_code("zh-Hant"), Some(Code::ZhTW));
  assert_eq!(by_code("zh-Hans"), Some(Code::Zh));
  assert_eq!(by_code("ZH-cn"), Some(Code::Zh));
  assert_eq!(by_code("en-US"), Some(Code::En));
  assert_eq!(by_code("pt_BR"), Some(Code::Pt));
  assert_eq!(by_code("fil-PH"), Some(Code::Fil));
  assert_eq!(by_code(" xx "), None);
  assert_eq!(by_code(""), None);
}

#[test]
fn str_name() {
  assert_eq!(Code::ZhTW.as_str(), "zh-TW");
  assert_eq!(Code::ZhTW.as_ref(), "zh-TW");
  assert_eq!(Code::Zh.name(), "简体中文");
  for &code in Code::VARIANTS {
    assert!(!NAME[code as usize].is_empty(), "{code}");
  }
}

#[test]
fn code_with_name() {
  assert_eq!(code_with_name_if_exist("zh"), "zh(简体中文)");
  assert_eq!(code_with_name_if_exist("zh-TW"), "zh-TW(正體中文)");
  assert_eq!(code_with_name_if_exist("en-US"), "en(English)");
  assert_eq!(code_with_name_if_exist("not-a-code"), "not-a-code");
}
