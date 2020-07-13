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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    line_number: usize,
    line: String,
    pub(crate) kind: ParseErrorKind
}

impl ParseError {
    /// Construct a `ParseError` of the specified `kind`
    /// based on the current `ParseContext`.
    pub fn with_kind(kind: ParseErrorKind, ctx: &ParseContext) -> Self {
        Self {
            line_number: ctx.current_line_number().unwrap_or_default(),
            line: ctx.current_line().unwrap_or_default().into(),
            kind,
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

    pub fn unit_test_miscount(ctx: &ParseContext) -> Self {
        Self::with_kind(ParseErrorKind::UnitTestMiscount, ctx)
    }

    pub fn benchmark_miscount(ctx: &ParseContext) -> Self {
        Self::with_kind(ParseErrorKind::BenchmarkMiscount, ctx)
    }

    pub fn doc_test_miscount(ctx: &ParseContext) -> Self {
        Self::with_kind(ParseErrorKind::DocTestMiscount, ctx)
    }
}
