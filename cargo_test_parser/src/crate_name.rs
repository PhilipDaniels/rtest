use crate::parse_error::ParseError;
use crate::{
    parse_context::ParseContext,
    utils::{exclusive_split_at_index, is_valid_uuid},
};

/// Represents the name parsed from a 'Running' line, such as
///   Running /home/phil/repos/rtest/target/debug/deps/example_lib_tests-9bdf7ee7378a8684
/// `full_name` is everything, the `uuid` is the bit at the end, and `pretty_name` is everything
/// up to the guid.
#[derive(Debug, Clone)]
pub struct CrateName<'a> {
    pub full_name: &'a str,
    pub uuid: &'a str,
    pub pretty_name: &'a str,
}

impl<'a> CrateName<'a> {
    /// Construct a new `CrateName`, parsing out the component bits.
    /// Returns an error if the name does not end in a UUID.
    pub(crate) fn parse<'ctx>(
        full_name: &'a str,
        ctx: &'ctx ParseContext,
    ) -> Result<CrateName<'a>, ParseError> {
        match full_name.rfind('-') {
            Some(idx) => {
                let (pretty_name, uuid) = exclusive_split_at_index(full_name, idx);
                let uuid = is_valid_uuid(uuid, ctx)?;

                Ok(Self {
                    full_name,
                    uuid,
                    pretty_name,
                })
            }
            None => Err(ParseError::malformed_crate_name(ctx)),
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
    fn new_for_empty_full_name() {
        let result = CrateName::parse("", &make_ctx()).unwrap_err();
        assert_eq!(result.kind, ParseErrorKind::MalformedCrateName);
    }

    #[test]
    fn new_for_full_name_with_no_guid() {
        let result = CrateName::parse("/long/path", &make_ctx()).unwrap_err();
        assert_eq!(result.kind, ParseErrorKind::MalformedCrateName);
    }

    #[test]
    fn new_for_full_name_with_valid_guid() {
        let result = CrateName::parse("/long/path-9bdf7ee7378a8684", &make_ctx()).unwrap();
        assert_eq!(result.full_name, "/long/path-9bdf7ee7378a8684");
        assert_eq!(result.pretty_name, "/long/path");
        assert_eq!(result.uuid, "9bdf7ee7378a8684");
    }
}
