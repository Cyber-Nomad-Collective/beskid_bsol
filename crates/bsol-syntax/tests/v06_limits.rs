//! Syntax budgets agree with native `Core.Bsol.Parser`: same defaults, same kinds, same messages.
use bsol_syntax::{
    parse_bsol_document, parse_bsol_document_with_limits, BsolError, BsolLimitKind, BsolLimits,
};

fn limits(edit: impl FnOnce(&mut BsolLimits)) -> BsolLimits {
    let mut limits = BsolLimits::default();
    edit(&mut limits);
    limits
}

fn rejects(source: &str, limits: &BsolLimits, kind: BsolLimitKind, message: &str) {
    let error = parse_bsol_document_with_limits(source, limits)
        .expect_err(&format!("{kind} rejects {source:?}"));
    assert_eq!(error.limit_kind(), Some(kind), "{error}");
    let BsolError::Limit {
        message: actual, ..
    } = &error
    else {
        unreachable!()
    };
    assert_eq!(*actual, message, "{error}");
}

fn accepts(source: &str, limits: &BsolLimits) {
    parse_bsol_document_with_limits(source, limits)
        .unwrap_or_else(|error| panic!("{source:?} is within budget: {error}"));
}

#[test]
fn defaults_match_the_native_descriptor() {
    let defaults = BsolLimits::default();
    assert_eq!(defaults.max_input_bytes, 8_388_608);
    assert_eq!(defaults.max_depth, 128);
    assert_eq!(defaults.max_nodes, 100_000);
    assert_eq!(defaults.max_scalar_bytes, 1_048_576);
}

#[test]
fn input_budget() {
    let source = "a { b = 1 }";
    accepts(source, &limits(|l| l.max_input_bytes = source.len()));
    rejects(
        source,
        &limits(|l| l.max_input_bytes = source.len() - 1),
        BsolLimitKind::InputLimit,
        "input byte limit",
    );
}

#[test]
fn invalid_descriptor() {
    rejects(
        "a { }",
        &limits(|l| l.max_depth = 0),
        BsolLimitKind::InvalidSyntax,
        "invalid parser limits",
    );
    rejects(
        "a { }",
        &limits(|l| l.max_nodes = 0),
        BsolLimitKind::InvalidSyntax,
        "invalid parser limits",
    );
}

#[test]
fn nesting_budget_counts_statements_assignments_and_values() {
    // root block 1, assignment 2, list 3, number 4
    let source = "a { b = [1] }";
    accepts(source, &limits(|l| l.max_depth = 4));
    rejects(
        source,
        &limits(|l| l.max_depth = 3),
        BsolLimitKind::DepthLimit,
        "BSOL nesting limit",
    );
    // map value 3, entry assignment 4, entry value 5
    let map = "a { b = { c = 1 } }";
    accepts(map, &limits(|l| l.max_depth = 5));
    rejects(
        map,
        &limits(|l| l.max_depth = 4),
        BsolLimitKind::DepthLimit,
        "BSOL nesting limit",
    );
}

fn nested_maps(levels: usize) -> String {
    format!(
        "a {{ b = {}1{} }}",
        "{ c = ".repeat(levels),
        " }".repeat(levels)
    )
}

#[test]
fn deep_nesting_is_rejected_without_unbounded_recursion() {
    let deep = format!(
        "a {{ b = {}1{} }}",
        "[".repeat(100_000),
        "]".repeat(100_000)
    );
    rejects(
        &deep,
        &BsolLimits::default(),
        BsolLimitKind::DepthLimit,
        "BSOL nesting limit",
    );
    rejects(
        &nested_maps(100_000),
        &BsolLimits::default(),
        BsolLimitKind::DepthLimit,
        "BSOL nesting limit",
    );
    // the innermost value of k nested maps sits at depth 3 + 2k
    parse_bsol_document(&nested_maps(62)).expect("nesting depth 127 is admitted");
    rejects(
        &nested_maps(63),
        &BsolLimits::default(),
        BsolLimitKind::DepthLimit,
        "BSOL nesting limit",
    );
}

#[test]
fn node_budget_counts_nodes_and_comments() {
    // block, two assignments and two values
    let source = "a { b = 1 c = 2 }";
    accepts(source, &limits(|l| l.max_nodes = 5));
    rejects(
        source,
        &limits(|l| l.max_nodes = 4),
        BsolLimitKind::NodeLimit,
        "syntax node limit",
    );
    let commented = "a { b = 1 c = 2 }\n# trailing";
    accepts(commented, &limits(|l| l.max_nodes = 6));
    rejects(
        commented,
        &limits(|l| l.max_nodes = 5),
        BsolLimitKind::NodeLimit,
        "comment limit",
    );
}

#[test]
fn scalar_budget_per_token_kind() {
    let three = limits(|l| l.max_scalar_bytes = 3);
    accepts("abc { }", &three);
    rejects(
        "abcd { }",
        &three,
        BsolLimitKind::ScalarLimit,
        "identifier byte limit",
    );
    accepts("a { b = 123 }", &three);
    rejects(
        "a { b = 1234 }",
        &three,
        BsolLimitKind::ScalarLimit,
        "number byte limit",
    );
    accepts(r#"a { b = "abc" }"#, &three);
    rejects(
        r#"a { b = "abcd" }"#,
        &three,
        BsolLimitKind::ScalarLimit,
        "decoded string byte limit",
    );
    accepts(r#"a { b = "ab\n" }"#, &three);
    rejects(
        r#"a { b = "abc\n" }"#,
        &three,
        BsolLimitKind::ScalarLimit,
        "decoded scalar byte limit",
    );
    rejects(
        r#"a "abcd" { }"#,
        &three,
        BsolLimitKind::ScalarLimit,
        "decoded string byte limit",
    );

    let eight = limits(|l| l.max_scalar_bytes = 8);
    accepts("a { t = list[i8] }", &eight);
    rejects(
        "a { t = list[int] }",
        &eight,
        BsolLimitKind::ScalarLimit,
        "type token byte limit",
    );

    let five = limits(|l| l.max_scalar_bytes = 5);
    accepts("a { r = @ab.cd/x }", &five);
    rejects(
        "a { r = @ab.cde/x }",
        &five,
        BsolLimitKind::ScalarLimit,
        "reference qualifier byte limit",
    );
}

#[test]
fn type_token_nesting_is_bounded_separately() {
    let three = limits(|l| l.max_depth = 3);
    accepts("a { t = list[list[list[int]]] }", &three);
    rejects(
        "a { t = list[list[list[list[int]]]] }",
        &three,
        BsolLimitKind::DepthLimit,
        "type expression nesting limit",
    );
}

#[test]
fn default_scalar_limit_is_exact() {
    let at = format!("a {{ b = \"{}\" }}", "x".repeat(1_048_576));
    parse_bsol_document(&at).expect("a 1 MiB string is admitted");
    let over = format!("a {{ b = \"{}\" }}", "x".repeat(1_048_577));
    let error = parse_bsol_document(&over).expect_err("one byte over the default scalar limit");
    assert_eq!(error.limit_kind(), Some(BsolLimitKind::ScalarLimit));
    assert!(
        error.to_string().contains("ScalarLimit")
            && error.to_string().contains("decoded string byte limit")
    );
}
