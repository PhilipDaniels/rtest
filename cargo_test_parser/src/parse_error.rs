use crate::parse_context::ParseContext;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseErrorKind {
    ExtraInput,
    UnexpectedEoF,
    MalformedCrateName,
    MalformedUuid,
    UnitTestMiscount,
    BenchmarkMiscount,
    DocTestMiscount,
    MalformedDocTestLine,
    SectionOverrun,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    line_number: usize,
    line: String,
    pub(crate) kind: ParseErrorKind,
    message: String,
}

impl ParseError {
    /// Construct a `ParseError` of the specified `kind`
    /// based on the current `ParseContext`.
    pub fn with_kind(kind: ParseErrorKind, ctx: &ParseContext) -> Self {
        Self {
            line_number: ctx.current_line_number().unwrap_or_default(),
            line: ctx.current_line().unwrap_or_default().into(),
            kind,
            message: String::default(),
        }
    }

    pub fn with_message(kind: ParseErrorKind, ctx: &ParseContext, message: String) -> Self {
        Self {
            line_number: ctx.current_line_number().unwrap_or_default(),
            line: ctx.current_line().unwrap_or_default().into(),
            kind,
            message,
        }
    }

    /// Construct a `ParseError` of kind `ParseErrorKind::ExtraInput`
    /// based on the current `ParseContext`.
    pub fn extra_input(ctx: &ParseContext) -> Self {
        Self::with_kind(ParseErrorKind::ExtraInput, ctx)
    }

    /// Construct a `ParseError` of kind `ParseErrorKind::UnexpectedEoF`
    /// based on the current `ParseContext`.
    pub fn unexpected_eof(ctx: &ParseContext) -> Self {
        Self::with_kind(ParseErrorKind::UnexpectedEoF, ctx)
    }

    /// Construct a `ParseError` of kind `ParseErrorKind::MalformedCrateName`
    /// based on the current `ParseContext`.
    pub fn malformed_crate_name(ctx: &ParseContext) -> Self {
        Self::with_kind(ParseErrorKind::MalformedCrateName, ctx)
    }

    /// Construct a `ParseError` of kind `ParseErrorKind::MalformedUuid`
    /// based on the current `ParseContext`.
    pub fn malformed_uuid(ctx: &ParseContext) -> Self {
        Self::with_kind(ParseErrorKind::MalformedUuid, ctx)
    }

    /// Construct a `ParseError` of kind `ParseErrorKind::UnitTestMiscount`
    /// based on the current `ParseContext`.
    pub fn unit_test_miscount(ctx: &ParseContext, actual_test_count: usize) -> Self {
        Self::with_message(ParseErrorKind::UnitTestMiscount, ctx, format!("Actual found test count: {}", actual_test_count))
    }

    /// Construct a `ParseError` of kind `ParseErrorKind::BenchmarkMiscount`
    /// based on the current `ParseContext`.
    pub fn benchmark_miscount(ctx: &ParseContext) -> Self {
        Self::with_kind(ParseErrorKind::BenchmarkMiscount, ctx)
    }

    /// Construct a `ParseError` of kind `ParseErrorKind::DocTestMiscount`
    /// based on the current `ParseContext`.
    pub fn doc_test_miscount(ctx: &ParseContext) -> Self {
        Self::with_kind(ParseErrorKind::DocTestMiscount, ctx)
    }

    /// Construct a `ParseError` of kind `ParseErrorKind::MalformedDocTestLine`
    /// based on the current `ParseContext`.
    pub fn malformed_doc_test_line(ctx: &ParseContext) -> Self {
        Self::with_kind(ParseErrorKind::MalformedDocTestLine, ctx)
    }

    /// Construct a `ParseError` of kind `ParseErrorKind::SectionOverrun`
    /// based on the current `ParseContext`.
    pub fn section_overrun(ctx: &ParseContext) -> Self {
        Self::with_kind(ParseErrorKind::SectionOverrun, ctx)
    }
}
