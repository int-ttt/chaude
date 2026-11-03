use chaude_scripts::charcount::count_text;

#[test]
fn bom_and_crlf_match_plain_lf() {
    let plain = "# 제목\n\n본문 한글 test\n";
    let bom_crlf = "\u{feff}# 제목\r\n\r\n본문 한글 test\r\n";
    let want = count_text(plain, "", 0).unwrap();
    assert_eq!(count_text(bom_crlf, "", 0).unwrap(), want);
    assert!(want.contains("공백 포함(줄바꿈 포함): 16"), "{want}");
}

#[test]
fn bom_only_is_stripped() {
    let got = count_text("\u{feff}abc", "", 0).unwrap();
    assert!(got.contains("공백 포함(줄바꿈 포함): 3"), "{got}");
    assert!(got.contains("바이트(UTF-8): 3 "), "{got}");
}

#[test]
fn section_limits_to_heading_body() {
    let doc = "# A\naaa\n## B\nbbbb\n# C\ncc\n";
    let got = count_text(doc, "B", 0).unwrap();
    assert!(got.contains("공백 포함(줄바꿈 포함): 4"), "{got}");
}

#[test]
fn missing_section_is_an_error() {
    let err = count_text("# A\n", "Z", 0).unwrap_err();
    assert_eq!(err, "section not found: Z");
}

#[test]
fn limit_reports_over_and_ok() {
    let over = count_text("abcd", "", 3).unwrap();
    assert!(over.contains("제한 3: OVER (-1)"), "{over}");
    let ok = count_text("abcd", "", 10).unwrap();
    assert!(ok.contains("제한 10: OK (+6)"), "{ok}");
}
