//! Shared Core.Text.Regex grammar/matching contract; no arbitrary regexcrate semantics.
use bsol::{
    load_profile_from_source, parse_bsol_document, parse_bsol_document_with_limits, validate,
    BsolLimitKind, BsolLimits,
};
fn fixture(name: &str, kind: &str) -> String {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../conformance/cases/schema-patterns");
    std::fs::read_to_string(root.join(format!("{name}.{kind}.bsol"))).unwrap()
}
fn summary(error: &impl std::fmt::Display) -> String {
    let text = error.to_string();
    format!(
        "{} [diagnostic bytes={}]",
        text.chars().take(180).collect::<String>(),
        text.len()
    )
}
fn check(name: &str, expected: &str) {
    let profile_source = fixture(name, "profile");
    let profile = load_profile_from_source(&profile_source);
    if expected == "InvalidPattern" {
        let error = profile.expect_err("invalid/unsupported regex grammar must reject profile");
        assert!(
            error.to_string().contains("InvalidPattern"),
            "checked invalidpattern diagnostic: {error}"
        );
        return;
    }
    let profile = profile.expect("supported regexgrammar must load");
    let source = fixture(name, "input");
    // Matches native `CheckPatternCase`: the over-limit subject needs an explicit syntax scalar
    // raise to reach pattern validation; the default policy rejects it as a ScalarLimit.
    let document = if name == "subject-over-limit" {
        let limits = BsolLimits {
            max_scalar_bytes: 1_048_577,
            ..BsolLimits::default()
        };
        parse_bsol_document_with_limits(&source, &limits)
    } else {
        parse_bsol_document(&source)
    }
    .expect("subject is valid quoted BSOL");
    let result = validate(&document, &profile);
    if expected == "ok" {
        result.unwrap_or_else(|error| panic!("shared case {name} must match: {}", summary(&error)));
    } else {
        let error = result.expect_err("shared pattern rejection must be checked");
        assert!(
            error.to_string().contains(expected),
            "expected explicit {expected}: {}",
            summary(&error)
        );
        let (start, end) = error
            .manifest_source_span()
            .expect("constraint error source-spanned");
        assert!(start < end && end <= source.len());
        assert!(error.manifest_source_line().is_some());
    }
}
#[test]
fn shared_pattern_anchored_match() {
    check("anchored-match", "ok");
}
#[test]
fn shared_pattern_anchored_reject() {
    check("anchored-reject", "PatternMismatch");
}
#[test]
fn shared_pattern_prefix_match() {
    check("prefix-match", "ok");
}
#[test]
fn shared_pattern_substring_reject() {
    check("substring-reject", "PatternMismatch");
}
#[test]
fn shared_pattern_alternation() {
    check("alternation", "ok");
}
#[test]
fn shared_pattern_group_plus() {
    check("group-plus", "ok");
}
#[test]
fn shared_pattern_class_range() {
    check("class-range", "ok");
}
#[test]
fn shared_pattern_negated_class() {
    check("negated-class", "ok");
}
#[test]
fn shared_pattern_escaped_digit() {
    check("escaped-digit", "ok");
}
#[test]
fn shared_pattern_escaped_literal() {
    check("escaped-literal", "ok");
}
#[test]
fn shared_pattern_grammar_escaped_brace() {
    check("grammar-escaped-brace", "ok");
}
#[test]
fn shared_pattern_quantified_sequence_backtrack() {
    check("quantified-sequence-backtrack", "ok");
}
#[test]
fn shared_pattern_empty_match_reject() {
    check("empty-match-reject", "PatternMismatch");
}
#[test]
fn shared_pattern_optional_zero_occurrence() {
    check("optional-zero-occurrence", "ok");
}
#[test]
fn shared_pattern_star_zero_occurrence() {
    check("star-zero-occurrence", "ok");
}
#[test]
fn shared_pattern_invalid_open_group() {
    check("invalid-open-group", "InvalidPattern");
}
#[test]
fn shared_pattern_invalid_empty_alternative() {
    check("invalid-empty-alternative", "InvalidPattern");
}
#[test]
fn shared_pattern_unsupported_lookaround() {
    check("unsupported-lookaround", "InvalidPattern");
}
#[test]
fn shared_pattern_unsupported_backreference() {
    check("unsupported-backreference", "InvalidPattern");
}
#[test]
fn shared_pattern_unsupported_counted() {
    check("unsupported-counted", "InvalidPattern");
}
#[test]
fn shared_pattern_subject_at_limit() {
    check("subject-at-limit", "ok");
}
#[test]
fn shared_pattern_subject_over_limit() {
    check("subject-over-limit", "PatternSubjectLimit");
}
#[test]
fn shared_pattern_subject_over_limit_rejects_under_default_syntax_limits() {
    let source = fixture("subject-over-limit", "input");
    let error = parse_bsol_document(&source).expect_err("default scalar limit rejects the subject");
    assert_eq!(error.limit_kind(), Some(BsolLimitKind::ScalarLimit));
    assert!(
        error.to_string().contains("decoded string byte limit"),
        "native diagnostic message: {error}"
    );
    parse_bsol_document(&fixture("subject-at-limit", "input"))
        .expect("subject at the limit parses");
}
#[test]
fn shared_pattern_nested_work_limit() {
    check("nested-work-limit", "PatternWorkLimit");
}

#[test]
fn shared_pattern_unicode_dot_accent() {
    check("unicode-dot-accent", "ok");
}

#[test]
fn shared_pattern_unicode_dot_astral() {
    check("unicode-dot-astral", "ok");
}

#[test]
fn shared_pattern_unicode_class_accent() {
    check("unicode-class-accent", "ok");
}

#[test]
fn shared_pattern_unicode_class_range() {
    check("unicode-class-range", "ok");
}

#[test]
fn shared_pattern_unicode_class_astral_range() {
    check("unicode-class-astral-range", "ok");
}

#[test]
fn shared_pattern_unicode_reject_two_scalars() {
    check("unicode-reject-two-scalars", "PatternMismatch");
}

#[test]
fn shared_pattern_unicode_class_reject() {
    check("unicode-class-reject", "PatternMismatch");
}

#[test]
fn unicode_pattern_rejection_spans_are_exact_utf8_assignment_boundaries() {
    let profile =
        load_profile_from_source(&fixture("unicode-reject-two-scalars", "profile")).unwrap();
    let source = fixture("unicode-reject-two-scalars", "input");
    let document = parse_bsol_document(&source).unwrap();
    let error = validate(&document, &profile).expect_err("two scalars reject dot anchor");
    let (start, end) = error.manifest_source_span().unwrap();
    assert!(source.is_char_boundary(start) && source.is_char_boundary(end));
    assert_eq!(&source[start..end], "value = \"é😀\"");
}
