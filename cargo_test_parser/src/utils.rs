use crate::{parse_context::ParseContext, parse_error::ParseError};

/// Splits the input into the part before and the part after
/// the character at `idx` (that character is not included in
/// either part).
pub fn exclusive_split_at_index(data: &str, idx: usize) -> (&str, &str) {
    (&data[..idx], &data[idx + 1..])
}

/// Splits the input into the part before and including
/// the character at `idx`, and the part after that.
pub fn inclusive_split_at_index(data: &str, idx: usize) -> (&str, &str) {
    (&data[..idx + 1], &data[idx + 1..])
}

/// Checks to see whether a string contains a valid UUID.
/// The string is expected to be 16 chars long and contain
/// only hex digits, in upper or lower case, for example
/// "9bdf7ee7378a8684". This is the format output by cargo.
pub fn is_valid_uuid<'a, 'ctx>(
    data: &'a str,
    ctx: &'ctx ParseContext,
) -> Result<&'a str, ParseError> {

    // TODO: Consider replacing this with a UUID crate if
    // cargo ever shows signs of changing their output format.
    if data.len() == 16 {
        let all_hex = data.chars().all(|c| c.is_ascii_hexdigit());
        if all_hex {
            return Ok(data);
        }
    }

    return Err(ParseError::malformed_uuid(ctx));
}

/// Parses a leading integer from a string. Does not cope with
/// negative numbers.
pub fn parse_leading_usize(data: &str) -> Option<usize> {
    let data = match data.find(|c: char| !c.is_ascii_digit()) {
        Some(idx) => {
            &data[0..idx]
        }
        None => data
    };

    dbg!(&data);
    data.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parse_error::ParseErrorKind, ParseContext};

    fn make_ctx() -> ParseContext<'static> {
        ParseContext::new("")
    }

    #[test]
    fn is_valid_uuid_for_empty_string() {
        let result = is_valid_uuid("", &make_ctx()).unwrap_err();
        assert_eq!(result.kind, ParseErrorKind::MalformedUuid);
    }

    #[test]
    fn is_valid_uuid_for_valid_uuid_lowercase() {
        let result = is_valid_uuid("9bdf7ee7378a8684", &make_ctx()).unwrap();
        assert_eq!(result, "9bdf7ee7378a8684");
    }

    #[test]
    fn is_valid_uuid_for_valid_uuid_uppercase() {
        let result = is_valid_uuid("9BDF7EE7378A8684", &make_ctx()).unwrap();
        assert_eq!(result, "9BDF7EE7378A8684");
    }

    #[test]
    fn is_valid_uuid_for_start_padded_uuid() {
        let result = is_valid_uuid("-9bdf7ee7378a8684", &make_ctx()).unwrap_err();
        assert_eq!(result.kind, ParseErrorKind::MalformedUuid);
    }

    #[test]
    fn is_valid_uuid_for_end_padded_uuid() {
        let result = is_valid_uuid("9bdf7ee7378a8684\n", &make_ctx()).unwrap_err();
        assert_eq!(result.kind, ParseErrorKind::MalformedUuid);
    }

    #[test]
    fn exclusive_split_at_index_for_single_char_data() {
        let (lhs, rhs) = exclusive_split_at_index("a", 0);
        assert_eq!(lhs, "");
        assert_eq!(rhs, "");
    }

    #[test]
    fn exclusive_split_at_index_nominal_case() {
        assert_eq!(exclusive_split_at_index("a-b", 1), ("a", "b"));
        assert_eq!(exclusive_split_at_index("abc-def", 3), ("abc", "def"));
    }

    #[test]
    fn inclusive_split_at_index_for_single_char_data() {
        let (lhs, rhs) = inclusive_split_at_index("a", 0);
        assert_eq!(lhs, "a");
        assert_eq!(rhs, "");
    }

    #[test]
    fn inclusive_split_at_index_nominal_case() {
        assert_eq!(inclusive_split_at_index("a-b", 1), ("a-", "b"));
        assert_eq!(inclusive_split_at_index("abc-def", 3), ("abc-", "def"));
    }

    #[test]
    fn parse_leading_usize_for_empty_data() {
        assert!(parse_leading_usize("").is_none());
    }

    #[test]
    fn parse_leading_usize_for_bad_data() {
        assert!(parse_leading_usize("-3").is_none());
        assert!(parse_leading_usize("abc12").is_none());
    }

    #[test]
    fn parse_leading_usize_for_good_data() {
        assert_eq!(parse_leading_usize("3"), Some(3));
        assert_eq!(parse_leading_usize("122abc"), Some(122));
        assert_eq!(parse_leading_usize("43 tests"), Some(43));
    }
}
