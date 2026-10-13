//! Span controls exercise global byte offsets through assignment/inline reparsing.
use bsol_syntax::{BsolBlock, BsolItem, BsolSpan, BsolValue, parse_bsol_document};

fn check_span(source: &str, span: BsolSpan) {
    assert!(span.start <= span.end && span.end <= source.len());
    assert!(source.is_char_boundary(span.start) && source.is_char_boundary(span.end));
    assert_eq!(
        span.line,
        1 + source.as_bytes()[..span.start]
            .iter()
            .filter(|byte| **byte == b'\n')
            .count()
    );
}

fn check_block(source: &str, block: &BsolBlock) {
    check_span(source, block.span);
    if let Some(label) = &block.label {
        check_span(source, label.span);
    }
    for attr in &block.attrs {
        check_span(source, attr.span);
    }
    for item in &block.items {
        match item {
            BsolItem::Block(block) => check_block(source, block),
            BsolItem::Assignment(assignment) => {
                check_span(source, assignment.span);
                for attr in &assignment.attrs {
                    check_span(source, attr.span);
                }
                match &assignment.value {
                    BsolValue::Number(number) => check_span(source, number.span),
                    BsolValue::QuotedString(value) => check_span(source, value.span),
                    BsolValue::InlineBlock(block) => check_block(source, block),
                    BsolValue::InlineMap(map) => {
                        check_span(source, map.span);
                        for entry in &map.entries {
                            check_span(source, entry.span);
                        }
                    }
                    BsolValue::BracketList(list) => {
                        check_span(source, list.span);
                        for item in &list.items {
                            if let bsol_syntax::BsolListItem::Number(number) = item {
                                check_span(source, number.span);
                            }
                        }
                    }
                    BsolValue::Ref(reference) => check_span(source, reference.span),
                    _ => {}
                }
            }
        }
    }
}

#[test]
fn v06_span_global_offsets_unicode_crlf_and_inline_reentry() {
    let source = "// α comment\r\n@Root(flag = true) root \"雪\" {\r\n  text = \"escaped\\nvalue\"\r\n  numbers = [1,\r\n    2]\r\n  map = { alpha = 3,\r\n    beta = 4 }\r\n  child = node \"inner\" {\r\n    count = 5\r\n  }\r\n}\r\nother { value = 6 }\r\n";
    let document = parse_bsol_document(source).unwrap();
    for block in &document.blocks {
        check_block(source, block);
    }
    assert_eq!(document.blocks[0].span.line, 2);
    assert_eq!(document.blocks[1].span.line, 12);
    let BsolItem::Assignment(child) = &document.blocks[0].items[3] else {
        panic!("child assignment")
    };
    let BsolValue::InlineBlock(child) = &child.value else {
        panic!("inline node")
    };
    let BsolItem::Assignment(count) = &child.items[0] else {
        panic!("count assignment")
    };
    let BsolValue::Number(number) = &count.value else {
        panic!("number")
    };
    assert_eq!(&source[number.span.start..number.span.end], "5");
    assert_eq!(number.span.line, 9);
}

#[test]
fn v06_span_error_global_offset_and_sequential_document_isolation() {
    let source = "// 雪\r\nroot {\r\n value = 1e999\r\n}";
    let error = parse_bsol_document(source).unwrap_err();
    let (start, end) = error.source_span().unwrap();
    assert_eq!(&source[start..end], "1e999");
    assert_eq!(error.source_line(), Some(3));
    let next = parse_bsol_document("root { value = 7 }").unwrap();
    assert_eq!(next.blocks[0].span.line, 1);
    check_block("root { value = 7 }", &next.blocks[0]);
    let eof = "// 雪\r\nroot {\r\n value = 1";
    let error = parse_bsol_document(eof).unwrap_err();
    assert_eq!(
        error.source_span(),
        Some((eof.find('{').unwrap(), eof.len()))
    );
    assert_eq!(error.source_line(), Some(2));
}

#[test]
fn v06_span_large_parse_timing_capture_preserves_full_work_case() {
    let values = std::iter::repeat_n("0", 60_000)
        .collect::<Vec<_>>()
        .join(",");
    let source = format!("config {{ first = [{values}] second = [{values}] }}");
    assert!(source.len() < 1_048_576);
    // 120000 list values exceed the default 100000-node syntax budget; raise it explicitly.
    let syntax = bsol_syntax::BsolLimits {
        max_nodes: 200_000,
        ..bsol_syntax::BsolLimits::default()
    };
    let started = std::time::Instant::now();
    let document = bsol_syntax::parse_bsol_document_with_limits(&source, &syntax).unwrap();
    eprintln!(
        "v06_span_large_parse bytes={} values=120000 elapsed_ms={}",
        source.len(),
        started.elapsed().as_millis()
    );
    for item in &document.blocks[0].items {
        let BsolItem::Assignment(assignment) = item else {
            panic!("assignment")
        };
        let BsolValue::BracketList(list) = &assignment.value else {
            panic!("list")
        };
        assert_eq!(list.items.len(), 60_000);
        let bsol_syntax::BsolListItem::Number(last) = list.items.last().unwrap() else {
            panic!("numeric item")
        };
        assert_eq!(&source[last.span.start..last.span.end], "0");
        assert_eq!(last.span.line, 1);
    }
}
