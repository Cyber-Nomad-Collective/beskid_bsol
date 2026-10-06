//! Caller policy and nested validation share one bounded pattern authority.
use bsol::{
    PatternLimits, load_profile_from_source, parse_bsol_document, validate_with_pattern_limits,
};

fn run(pattern: &str, value: &str, limits: PatternLimits) -> Result<(), bsol::BsolError> {
    let pattern = pattern.replace('\\', "\\\\").replace('"', "\\\"");
    let profile = load_profile_from_source(&format!(
        "profile \"limits.v2\" {{ version = 2 rule \"config\" {{ match = keyword keyword = config label = forbidden cardinality = one field \"value\" {{ type = quoted required = true pattern = \"{pattern}\" }} }} }}"
    ))?;
    let document = parse_bsol_document(&format!("config {{ value = \"{value}\" }}")).unwrap();
    validate_with_pattern_limits(&document, &profile, &limits).map(|_| ())
}
fn limits(work: usize, depth: usize, subject: usize) -> PatternLimits {
    PatternLimits {
        maxSubjectBytes: subject,
        maxWork: work,
        maxDepth: depth,
    }
}
fn rejection(result: Result<(), bsol::BsolError>, code: &str) {
    let error = result.expect_err("explicit policy rejection required");
    assert!(error.to_string().contains(code), "expected {code}: {error}");
    assert!(
        error.manifest_source_span().is_some(),
        "policy error source-spanned"
    );
}
#[test]
fn caller_lowered_budget_fails_before_atom_execution() {
    rejection(
        run("^abc$", "abc", limits(2, 64, 1048576)),
        "PatternWorkLimit",
    );
    run("^abc$", "abc", limits(100, 64, 1048576)).unwrap();
}
#[test]
fn caller_lowered_subject_and_depth_limits_are_checked() {
    rejection(run("abc", "abc", limits(100, 64, 2)), "PatternSubjectLimit");
    rejection(
        run("((abc))", "abc", limits(100, 1, 1048576)),
        "PatternDepthLimit",
    );
}
#[test]
fn caller_zero_work_is_deterministic_exhaustion() {
    rejection(run("a", "a", limits(0, 64, 1048576)), "PatternWorkLimit");
}
#[test]
fn caller_work_is_not_reset_between_inline_and_outer_fields() {
    let profile = load_profile_from_source("profile \"limits.v2\" { version = 2 rule \"config\" { match = keyword keyword = config label = forbidden cardinality = one field \"first\" { type = quoted pattern = \"abcdefgh\" required = true } field \"inside\" { type = inline(holder) required = true } } rule \"holder\" { match = keyword keyword = holder label = forbidden cardinality = many field \"second\" { type = quoted pattern = \"abcdefgh\" required = true } } }").unwrap();
    let document = parse_bsol_document(
        "config { first = \"abcdefgh\" inside = holder { second = \"abcdefgh\" } }",
    )
    .unwrap();
    run("abcdefgh", "abcdefgh", limits(50, 64, 1048576)).unwrap();
    let bounded = limits(50, 64, 1048576);
    let error = validate_with_pattern_limits(&document, &profile, &bounded)
        .expect_err("second nested pattern exhausts cumulative budget");
    assert!(error.to_string().contains("PatternWorkLimit"));
    assert!(
        error.manifest_source_span().unwrap().0 > 30,
        "failure belongs to second inline field, not the first pattern"
    );
    validate_with_pattern_limits(&document, &profile, &limits(1000, 64, 1048576)).unwrap();
}
#[test]
fn caller_work_is_not_reset_between_missing_typed_defaults() {
    let profile = load_profile_from_source("profile \"limits.v2\" { version = 2 rule \"config\" { match = keyword keyword = config label = forbidden cardinality = one field \"first\" { type = quoted pattern = \"abcdefgh\" default = \"abcdefgh\" } field \"second\" { type = quoted pattern = \"abcdefgh\" default = \"abcdefgh\" } } }").unwrap();
    let document = parse_bsol_document("config {}").unwrap();
    rejection(
        validate_with_pattern_limits(&document, &profile, &limits(50, 64, 1048576)).map(|_| ()),
        "PatternWorkLimit",
    );
}

#[test]
fn caller_limits_cannot_exceed_checked_address_bounds() {
    rejection(
        run("abc", "abc", limits(usize::MAX, 64, 1048576)),
        "InvalidPatternLimits",
    );
    rejection(
        run("abc", "abc", limits(100, usize::MAX, 1048576)),
        "InvalidPatternLimits",
    );
}

#[test]
fn unicode_pattern_subject_policy_counts_utf8_bytes_not_scalars() {
    run("^.$", "😀", limits(100, 64, 4)).unwrap();
    rejection(run("^.$", "😀", limits(100, 64, 3)), "PatternSubjectLimit");
    run("^.$", "é", limits(100, 64, 2)).unwrap();
    rejection(run("^.$", "é", limits(100, 64, 1)), "PatternSubjectLimit");
}

#[test]
fn exact_work_model_counts_initial_accept_and_only_inspected_class_ranges() {
    run("[abc]", "a", limits(11, 64, 1048576)).unwrap();
    rejection(
        run("[abc]", "a", limits(10, 64, 1048576)),
        "PatternWorkLimit",
    );
    run("[abc]", "c", limits(13, 64, 1048576)).unwrap();
    rejection(
        run("[abc]", "c", limits(12, 64, 1048576)),
        "PatternWorkLimit",
    );
}
