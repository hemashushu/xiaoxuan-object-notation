// Copyright (c) 2024 Hemashushu <hippospark@gmail.com>, All rights reserved.
//
// This Source Code Form is subject to the terms of
// the Mozilla Public License version 2.0 and additional exceptions,
// more details in file LICENSE, LICENSE.additional and CONTRIBUTING.

use crate::AsonError;

//                 /-- selection start
//                 |                 /-- selection length
//                 v                 v
// prefix -->   ...snippet_source_text...  <-- suffix
//                       ^^^^
//                       |  \-- length
//                       \----- offset
struct SnippetRange {
    prefix: bool,
    suffix: bool,
    selection_start: usize,
    selection_length: usize,
    offset: usize,
    length: usize,
}

fn calculate_snippet_range(
    original_selection_start: usize,
    original_selection_length: usize,
    source_total_length: usize,
) -> SnippetRange {
    const LEADING_LENGTH: usize = 15;
    const SNIPPET_LENGTH: usize = 40;

    let (prefix, selection_start, offset) =
        if source_total_length < SNIPPET_LENGTH || original_selection_start < LEADING_LENGTH {
            (false, 0, original_selection_start)
        } else if original_selection_start + SNIPPET_LENGTH > source_total_length {
            (
                true,
                source_total_length - SNIPPET_LENGTH,
                original_selection_start - (source_total_length - SNIPPET_LENGTH),
            )
        } else {
            (
                true,
                original_selection_start - LEADING_LENGTH,
                LEADING_LENGTH,
            )
        };

    let (suffix, selection_length) = if selection_start + SNIPPET_LENGTH >= source_total_length {
        (false, source_total_length - selection_start)
    } else {
        (true, SNIPPET_LENGTH)
    };

    let length = if offset + original_selection_length > selection_length {
        selection_length - offset
    } else {
        original_selection_length
    };

    SnippetRange {
        prefix,
        suffix,
        selection_start,
        selection_length,
        offset,
        length,
    }
}

fn generate_snippet_and_indented_detail(
    chars: &mut dyn Iterator<Item = char>,
    snippet_range: &SnippetRange,
    detail: &str,
) -> (String, String) {
    // build snippet
    let mut snippet = String::new();
    snippet.push_str("| ");
    if snippet_range.prefix {
        snippet.push_str("...");
    }
    let selection_chars = chars
        .skip(snippet_range.selection_start)
        .take(snippet_range.selection_length);
    let selection_string = selection_chars
        .map(|c| match c {
            '\n' => ' ',
            '\t' => ' ',
            _ => c,
        })
        .collect::<String>();
    snippet.push_str(&selection_string);
    if snippet_range.suffix {
        snippet.push_str("...");
    }

    // build indented detail
    let mut indented_detail = String::new();
    indented_detail.push_str("| ");
    if snippet_range.prefix {
        indented_detail.push_str("   ");
    }
    indented_detail.push_str(&" ".repeat(snippet_range.offset));
    indented_detail.push('^');
    if snippet_range.length > 0 {
        indented_detail.push_str(&"^".repeat(snippet_range.length - 1));
    } else {
        indented_detail.push_str("____");
    }
    indented_detail.push(' ');
    indented_detail.push_str(detail);

    (snippet, indented_detail)
}

impl AsonError {
    pub fn with_source(&self, source: &str) -> String {
        // print human readable error message with the source

        let source_total_length = source.chars().count();
        let mut chars = source.chars();

        // | leading length
        // v
        // |------|
        // xxxxxxxx.xxxxxxxxxxx  <-- snippet text
        // |------------------|
        // ^
        // | snippet length

        match self {
            AsonError::Message(msg) => msg.to_owned(),
            AsonError::UnexpectedEndOfDocument(detail) => {
                let msg = "Unexpected to reach the end of document.";
                let snippet_range =
                    calculate_snippet_range(source_total_length, 0, source_total_length);
                let (snippet, indented_detail) =
                    generate_snippet_and_indented_detail(&mut chars, &snippet_range, detail);
                format!("{}\n{}\n{}", msg, snippet, indented_detail)
            }
            AsonError::MessageWithLocation(detail, location) => {
                let msg = format!(
                    "Error at line: {}, column: {}",
                    location.line + 1,
                    location.column + 1
                );

                let snippet_range =
                    calculate_snippet_range(location.index, location.length, source_total_length);
                let (snippet, indented_detail) =
                    generate_snippet_and_indented_detail(&mut chars, &snippet_range, detail);
                format!("{}\n{}\n{}", msg, snippet, indented_detail)
            }
        }
    }
}

#[cfg(test)]
mod tests {

    use pretty_assertions::assert_eq;

    use crate::{location::Location, AsonError};

    #[test]
    fn test_error_with_source() {
        let source1 = "0123456789"; // 10 chars
        let source2 = "012345678_b12345678_c12345678_d12345678_e123456789"; // 50 chars
        let msg = "abcde";

        assert_eq!(AsonError::Message(msg.to_owned()).with_source(source1), msg);
        assert_eq!(AsonError::Message(msg.to_owned()).with_source(source2), msg);
    }

    #[test]
    fn test_error_with_source_and_unexpected_end_of_document() {
        let source1 = "0123456789"; // 10 chars
        let source2 = "012345678_b12345678_c12345678_d12345678_e123456789"; // 50 chars
        let msg = "abcde";

        assert_eq!(
            AsonError::UnexpectedEndOfDocument(msg.to_owned()).with_source(source1),
            r#"Unexpected to reach the end of document.
| 0123456789
|           ^____ abcde"#
        );

        assert_eq!(
            AsonError::UnexpectedEndOfDocument(msg.to_owned()).with_source(source2),
            r#"Unexpected to reach the end of document.
| ...b12345678_c12345678_d12345678_e123456789
|                                            ^____ abcde"#
        );
    }

    #[test]
    fn test_error_with_source_and_location() {
        let source1 = "0123456789"; // 10 chars
        let source2 = "012345678_b12345678_c12345678_d12345678_e123456789"; // 50 chars
        let msg = "abcde";

        // first

        assert_eq!(
            AsonError::MessageWithLocation(
                msg.to_owned(),
                Location::new_position(/*0,*/ 0, 11, 13)
            )
            .with_source(source1),
            r#"Error at line: 12, column: 14
| 0123456789
| ^____ abcde"#
        );

        assert_eq!(
            AsonError::MessageWithLocation(
                msg.to_owned(),
                Location::new_position(/*0,*/ 0, 11, 13)
            )
            .with_source(source2),
            r#"Error at line: 12, column: 14
| 012345678_b12345678_c12345678_d12345678_...
| ^____ abcde"#
        );

        // head

        assert_eq!(
            AsonError::MessageWithLocation(
                msg.to_owned(),
                Location::new_position(/*0,*/ 2, 11, 13)
            )
            .with_source(source1),
            r#"Error at line: 12, column: 14
| 0123456789
|   ^____ abcde"#
        );

        assert_eq!(
            AsonError::MessageWithLocation(
                msg.to_owned(),
                Location::new_position(/*0,*/ 15, 11, 13)
            )
            .with_source(source2),
            r#"Error at line: 12, column: 14
| ...b12345678_c12345678_d12345678_e123456789
|         ^____ abcde"#
        );

        // middle

        assert_eq!(
            AsonError::MessageWithLocation(
                msg.to_owned(),
                Location::new_position(/*0,*/ 5, 11, 13)
            )
            .with_source(source1),
            r#"Error at line: 12, column: 14
| 0123456789
|      ^____ abcde"#
        );

        assert_eq!(
            AsonError::MessageWithLocation(
                msg.to_owned(),
                Location::new_position(/*0,*/ 25, 11, 13)
            )
            .with_source(source2),
            r#"Error at line: 12, column: 14
| ...b12345678_c12345678_d12345678_e123456789
|                   ^____ abcde"#
        );

        // tail

        assert_eq!(
            AsonError::MessageWithLocation(
                msg.to_owned(),
                Location::new_position(/*0,*/ 8, 11, 13)
            )
            .with_source(source1),
            r#"Error at line: 12, column: 14
| 0123456789
|         ^____ abcde"#
        );

        assert_eq!(
            AsonError::MessageWithLocation(
                msg.to_owned(),
                Location::new_position(/*0,*/ 45, 11, 13)
            )
            .with_source(source2),
            r#"Error at line: 12, column: 14
| ...b12345678_c12345678_d12345678_e123456789
|                                       ^____ abcde"#
        );

        // last

        assert_eq!(
            AsonError::MessageWithLocation(
                msg.to_owned(),
                Location::new_position(/*0,*/ 10, 11, 13)
            )
            .with_source(source1),
            r#"Error at line: 12, column: 14
| 0123456789
|           ^____ abcde"#
        );

        assert_eq!(
            AsonError::MessageWithLocation(
                msg.to_owned(),
                Location::new_position(/*0,*/ 50, 11, 13)
            )
            .with_source(source2),
            r#"Error at line: 12, column: 14
| ...b12345678_c12345678_d12345678_e123456789
|                                            ^____ abcde"#
        );
    }

    #[test]
    fn test_error_with_source_and_range() {
        let source1 = "0123456789"; // 10 chars
        let source2 = "012345678_b12345678_c12345678_d12345678_e123456789"; // 50 chars
        let msg = "abcde";

        // first

        assert_eq!(
            AsonError::MessageWithLocation(
                msg.to_owned(),
                Location::new_range(/*0,*/ 0, 17, 19, 4)
            )
            .with_source(source1),
            r#"Error at line: 18, column: 20
| 0123456789
| ^^^^ abcde"#
        );

        assert_eq!(
            AsonError::MessageWithLocation(
                msg.to_owned(),
                Location::new_range(/*0,*/ 0, 17, 19, 8)
            )
            .with_source(source2),
            r#"Error at line: 18, column: 20
| 012345678_b12345678_c12345678_d12345678_...
| ^^^^^^^^ abcde"#
        );

        // head

        assert_eq!(
            AsonError::MessageWithLocation(
                msg.to_owned(),
                Location::new_range(/*0,*/ 2, 17, 19, 4)
            )
            .with_source(source1),
            r#"Error at line: 18, column: 20
| 0123456789
|   ^^^^ abcde"#
        );

        assert_eq!(
            AsonError::MessageWithLocation(
                msg.to_owned(),
                Location::new_range(/*0,*/ 15, 17, 19, 8)
            )
            .with_source(source2),
            r#"Error at line: 18, column: 20
| ...b12345678_c12345678_d12345678_e123456789
|         ^^^^^^^^ abcde"#
        );

        // middle

        assert_eq!(
            AsonError::MessageWithLocation(
                msg.to_owned(),
                Location::new_range(/*0,*/ 5, 17, 19, 4)
            )
            .with_source(source1),
            r#"Error at line: 18, column: 20
| 0123456789
|      ^^^^ abcde"#
        );

        assert_eq!(
            AsonError::MessageWithLocation(
                msg.to_owned(),
                Location::new_range(/*0,*/ 25, 17, 19, 8)
            )
            .with_source(source2),
            r#"Error at line: 18, column: 20
| ...b12345678_c12345678_d12345678_e123456789
|                   ^^^^^^^^ abcde"#
        );

        // tail

        assert_eq!(
            AsonError::MessageWithLocation(
                msg.to_owned(),
                Location::new_range(/*0,*/ 8, 17, 19, 4)
            )
            .with_source(source1),
            r#"Error at line: 18, column: 20
| 0123456789
|         ^^ abcde"#
        );

        assert_eq!(
            AsonError::MessageWithLocation(
                msg.to_owned(),
                Location::new_range(/*0,*/ 45, 17, 19, 8)
            )
            .with_source(source2),
            r#"Error at line: 18, column: 20
| ...b12345678_c12345678_d12345678_e123456789
|                                       ^^^^^ abcde"#
        );

        // last

        assert_eq!(
            AsonError::MessageWithLocation(
                msg.to_owned(),
                Location::new_range(/*0,*/ 10, 17, 19, 4)
            )
            .with_source(source1),
            r#"Error at line: 18, column: 20
| 0123456789
|           ^____ abcde"#
        );

        assert_eq!(
            AsonError::MessageWithLocation(
                msg.to_owned(),
                Location::new_range(/*0,*/ 50, 17, 19, 8)
            )
            .with_source(source2),
            r#"Error at line: 18, column: 20
| ...b12345678_c12345678_d12345678_e123456789
|                                            ^____ abcde"#
        );
    }
}
