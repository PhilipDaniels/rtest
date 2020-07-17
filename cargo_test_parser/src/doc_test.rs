use crate::{parse_context::ParseContext, parse_error::ParseError, utils::parse_leading_usize};

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DocTest<'a> {
    pub name: &'a str,
    pub line_number: usize,
    pub file_name: &'a str,
}

impl<'a> DocTest<'a> {
    /// Construct a new `DocTest` from a line of the form
    /// "src/lib.rs - passing_doctest (line 3): test".
    pub(crate) fn parse<'ctx>(
        line: &'a str,
        ctx: &'ctx ParseContext,
    ) -> Result<DocTest<'a>, ParseError> {
        let line = line.trim();
        if line.is_empty() {
            return Err(ParseError::malformed_doc_test_line(ctx));
        }

        match line.find(" - ") {
            Some(idx) => {
                let (file_name, remainder) = (&line[..idx], &line[idx + 3..]);
                let remainder = remainder.trim_end_matches(": test");

                match remainder.rfind(" (line ") {
                    Some(idx) => {
                        let (name, line_expr) = (&remainder[..idx], &remainder[idx + 7..]);
                        let line_number = match parse_leading_usize(line_expr) {
                            Some(n) => n,
                            None => return Err(ParseError::malformed_doc_test_line(ctx)),
                        };

                        return Ok(Self {
                            name,
                            line_number,
                            file_name,
                        });
                    }
                    None => return Err(ParseError::malformed_doc_test_line(ctx)),
                }
            }
            None => return Err(ParseError::malformed_doc_test_line(ctx)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_error::ParseErrorKind;

    fn make_ctx() -> ParseContext<'static> {
        ParseContext::new("")
    }

    #[test]
    fn parse_empty_line() {
        let result = DocTest::parse("", &make_ctx()).unwrap_err();
        assert_eq!(result.kind, ParseErrorKind::MalformedDocTestLine);
    }

    #[test]
    fn parse_line_without_separator() {
        let result = DocTest::parse("some line", &make_ctx()).unwrap_err();
        assert_eq!(result.kind, ParseErrorKind::MalformedDocTestLine);
    }

    #[test]
    fn parse_correct_line() {
        let result =
            DocTest::parse("src/lib.rs - passing_doctest (line 233): test", &make_ctx()).unwrap();
        assert_eq!(result.name, "passing_doctest");
        assert_eq!(result.file_name, "src/lib.rs");
        assert_eq!(result.line_number, 233);
    }
}
