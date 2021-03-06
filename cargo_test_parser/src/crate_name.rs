use crate::parse_error::ParseError;
use crate::{
    parse_context::ParseContext,
    utils::{exclusive_split_at_index, is_valid_uuid},
};

/// Represents the name parsed from a 'Running' line, such as
/// "Running /home/phil/repos/rtest/target/debug/deps/example_lib_tests-9bdf7ee7378a8684"
/// or the name parsed from a 'Doc-tests' line such as
/// "Doc-tests example_lib_tests".
#[derive(Debug, Clone)]
pub struct CrateName<'a> {
    /// The full name of the crate, as extracted from a 'Running' line, for example
    /// "Running /home/phil/repos/rtest/target/debug/deps/example_lib_tests-9bdf7ee7378a8684"
    pub full_name: &'a str,

    /// The UUID part of the `full_name`.
    pub uuid: &'a str,

    /// The name with the UUID removed, for example
    /// "/home/phil/repos/rtest/target/debug/deps/example_lib_tests".
    pub name: &'a str,

    /// The base part of the `name`, for example "example_lib_tests"
    /// from "/home/phil/repos/rtest/target/debug/deps/example_lib_tests".
    pub basename: &'a str,
}

impl<'a> CrateName<'a> {
    /// Construct a new `CrateName`, parsing out the component bits.
    /// Returns an error if the name does not end in a UUID.
    pub(crate) fn parse<'ctx>(
        full_name: &'a str,
        ctx: &'ctx ParseContext,
    ) -> Result<CrateName<'a>, ParseError> {
        let full_name = full_name.trim();
        if full_name.is_empty() {
            return Err(ParseError::malformed_crate_name(ctx));
        }

        match full_name.rfind('-') {
            Some(idx) => {
                let (name, uuid) = exclusive_split_at_index(full_name, idx);
                let uuid = is_valid_uuid(uuid, ctx)?;
                let basename = match name.rfind("/") {
                    Some(idx) => &name[idx + 1..],
                    None => name,
                };

                Ok(Self {
                    full_name,
                    uuid,
                    name,
                    basename,
                })
            }
            None => {
                // Just assume everything is the crate name. This can occur, for example
                // when using just "Doc-test some_crate_name".
                Ok(Self {
                    full_name,
                    uuid: "",
                    name: full_name,
                    basename: full_name,
                })
            }
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
    fn parse_empty_full_name() {
        let result = CrateName::parse("", &make_ctx()).unwrap_err();
        assert_eq!(result.kind, ParseErrorKind::MalformedCrateName);
    }

    #[test]
    fn parse_one_word_name_like_in_doc_tests() {
        let result = CrateName::parse("winterfell", &make_ctx()).unwrap();
        assert_eq!(result.basename, "winterfell");
        assert_eq!(result.uuid, "");
        assert_eq!(result.name, "winterfell");
        assert_eq!(result.full_name, "winterfell");
    }

    #[test]
    fn parse_full_name_with_no_guid() {
        let result = CrateName::parse("/long/path", &make_ctx()).unwrap();
        assert_eq!(result.basename, "/long/path");
        assert_eq!(result.uuid, "");
        assert_eq!(result.name, "/long/path");
        assert_eq!(result.full_name, "/long/path");
    }

    #[test]
    fn parse_full_name_with_multiple_components_and_valid_guid() {
        let result = CrateName::parse("/long/path-9bdf7ee7378a8684", &make_ctx()).unwrap();
        assert_eq!(result.full_name, "/long/path-9bdf7ee7378a8684");
        assert_eq!(result.name, "/long/path");
        assert_eq!(result.uuid, "9bdf7ee7378a8684");
        assert_eq!(result.basename, "path");
    }

    #[test]
    fn parse_full_name_with_single_component_and_valid_guid() {
        let result = CrateName::parse("/path-9bdf7ee7378a8684", &make_ctx()).unwrap();
        assert_eq!(result.full_name, "/path-9bdf7ee7378a8684");
        assert_eq!(result.name, "/path");
        assert_eq!(result.uuid, "9bdf7ee7378a8684");
        assert_eq!(result.basename, "path");
    }

    #[test]
    fn parse_full_name_with_no_leading_slash_and_valid_guid() {
        let result = CrateName::parse("path-9bdf7ee7378a8684", &make_ctx()).unwrap();
        assert_eq!(result.full_name, "path-9bdf7ee7378a8684");
        assert_eq!(result.name, "path");
        assert_eq!(result.uuid, "9bdf7ee7378a8684");
        assert_eq!(result.basename, "path");
    }
}
