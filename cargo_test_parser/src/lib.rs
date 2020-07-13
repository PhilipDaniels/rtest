mod parse_context;
mod parse_error;

use parse_context::ParseContext;
use parse_error::ParseError;

/*
- Recognise a unit test section
    - Starts ^[ws]Running CRATE_NAME-GUID$
    - Ends ^N tests, M benchmarks$
    - Recognise a unit test name
        - ^[MODULEPATH::TESTNAME]: test$

- Recognise a doc section
    - Starts ^[ws]Doc-tests CRATE_NAME$
    - Ends ^N tests, M benchmarks$
    - May be empty ("0 tests, 0 benchmarks")
    - Recognise a doc test name
        - ^[FILENAME].rs - TESTNAME (line N): test$
*/

/// Parses the output of `cargo test -- --list` and returns the result.
/// There will be one entry in the result for each crate that was
/// parsed. The parsing does not allocate any Strings, it only
/// borrows references to the input `data`.
pub fn parse_test_list(data: &str) -> Result<Vec<CrateTestList>, ParseError> {
    let mut result = Vec::new();
    let mut ctx = ParseContext::new(data);

    loop {
        match parse_crate_test_list(&mut ctx)? {
            Some(tests) => result.push(tests),
            None => break
        }
    }

    Ok(result)
}

/// Represents the set of tests in a single crate.
#[derive(Debug, Default, Clone)]
pub struct CrateTestList<'a> {
    pub crate_name: &'a str,
    pub unit_tests: Vec<&'a str>,
    pub doc_tests: Vec<DocTest<'a>>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DocTest<'a> {
    pub name: &'a str,
    pub line: usize,
    pub file: &'a str,
}

/// Parse a single `CrateTestList` from the input.
fn parse_crate_test_list<'ctx, 'a>(ctx: &'ctx mut ParseContext<'a>) -> Result<Option<CrateTestList<'a>>, ParseError> {
    let mut skip = ctx.skip_while(|line| !line.contains("Running "));

    loop {
        let line = skip.next();
        if line.is_none() {
            break;
        }
    }

    while let Some(line) = skip.next() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(parsed) = parse_unit_test(line) {

        } else if let Some(parsed) = parse_doc_test(line) {

        } else if let Some(parsed) = parse_test_summary_count(line) {

        } else {
            // Anything else, we consider ourselves to be at the end.
            break;
        }
    }

    Ok(None)
}

fn parse_unit_test(line: &str) -> Option<&str> {
    None
}

fn parse_doc_test(line: &str) -> Option<&str> {
    None
}

fn parse_test_summary_count(line: &str) -> Option<&str> {
    None
}





// fn parse_crate(data: &str) -> Result<ParsedData, ParseError> {
//     match data.find("Running ") {
//         Some(idx) => {
//             let data = &data[idx..];
//             let more_data = parse_crate_name(data)?;

//             let tests = CrateTests {
//                 crate_name: more_data.data.trim_end(),
//                 unit_tests: vec![],
//                 doc_tests: vec![],
//             };

//             return Ok(ParsedData::CrateTest {
//                 tests,
//                 remainder: more_data.remainder,
//             });
//         }
//         None => match data.trim().is_empty() {
//             true => return Ok(ParsedData::Done),
//             false => return Err(ParseError::ExtraInput(data.into())),
//         },
//     }
// }

// fn parse_crate_name(data: &str) -> Result<MoreData, ParseError> {
//     let data = data.trim_start_matches("Running ");
//     eat_to_next_linefeed_expect_more(data)
// }

// /// Eats up to the next linefeed '\n' character.
// /// The 'n' character is NOT removed, it is included in the output.
// fn eat_to_next_linefeed(data: &str) -> EatenData {
//     match data.find('\n') {
//         Some(idx) if idx == data.len() - 1 => EatenData::EndOfData { data },
//         Some(idx) => {
//             let (data, remainder) = inclusive_split_at_index(data, idx);
//             EatenData::more(data, remainder)
//         }
//         None => EatenData::EndOfData { data },
//     }
// }

// fn eat_to_next_linefeed_expect_more(data: &str) -> Result<MoreData, ParseError> {
//     match eat_to_next_linefeed(data) {
//         EatenData::EndOfData { data } => return Err(ParseError::UnexpectedEoF(data.into())),
//         EatenData::MoreData(more) => Ok(more),
//     }
// }

// /// Represents the name parsed from a 'Running' line, such as
// ///   Running /home/phil/repos/rtest/target/debug/deps/example_lib_tests-9bdf7ee7378a8684
// /// `full_name` is everything, the `uuid` is the bit at the end, and `pretty_name` is everything
// /// up to the guid.
// #[derive(Debug, Clone)]
// pub struct CrateName<'a> {
//     full_name: &'a str,
//     uuid: &'a str,
//     pretty_name: &'a str,
// }

// impl<'a> CrateName<'a> {
//     /// Construct a new `CrateName`, parsing out the component bits.
//     /// Returns an error if the name does not end in a UUID.
//     fn new(full_name: &'a str) -> Result<Self, ParseError> {
//         match full_name.rfind('-') {
//             Some(idx) => {
//                 let (pretty_name, uuid) = exclusive_split_at_index(full_name, idx);
//                 let uuid = is_valid_uuid(uuid)?;

//                 Ok(Self {
//                     full_name,
//                     uuid,
//                     pretty_name,
//                 })
//             }
//             None => Err(ParseError::MalformedCrateName(full_name.into())),
//         }
//     }
// }

/// Splits the input into the part before and the part after
/// the character at `idx` (that character is not included in
/// either part).
fn exclusive_split_at_index(data: &str, idx: usize) -> (&str, &str) {
    (&data[..idx], &data[idx + 1..])
}

/// Splits the input into the part before and including
/// the character at `idx`, and the part after that.
fn inclusive_split_at_index(data: &str, idx: usize) -> (&str, &str) {
    (&data[..idx + 1], &data[idx + 1..])
}

// fn is_valid_uuid(data: &str) -> Result<&str, ParseError> {
//     if data.len() == 16 {
//         let all_hex = data.chars().all(|c| c.is_ascii_hexdigit());
//         if all_hex {
//             return Ok(data);
//         }
//     }

//     return Err(ParseError::MalformedUuid(data.into()));
// }

#[derive(Debug, Clone, PartialEq, Eq)]
struct MoreData<'a> {
    data: &'a str,
    remainder: &'a str,
}

impl<'a> MoreData<'a> {
    /// Constructor for a `MoreData` instance.
    fn new(data: &'a str, remainder: &'a str) -> Self {
        Self { data, remainder }
    }
}

/// Represents the result of an 'eat' operation.
#[derive(Debug, Clone, PartialEq, Eq)]
enum EatenData<'a> {
    /// All the data was eaten, there is no more to follow.
    EndOfData { data: &'a str },
    /// Some of the data was eaten, but there is more to follow.
    MoreData(MoreData<'a>),
}

impl<'a> EatenData<'a> {
    /// Constructor for the EndOfData variant.
    fn end(data: &'a str) -> Self {
        Self::EndOfData { data: data }
    }

    /// Constructor the the MoreData variant.
    fn more(data: &'a str, remainder: &'a str) -> Self {
        Self::MoreData(MoreData::new(data, remainder))
    }
}

#[derive(Debug, Clone)]
pub enum ParsedData<'a> {
    Done,
    CrateTest {
        tests: CrateTestList<'a>,
        remainder: &'a str,
    },
}



#[cfg(test)]
static ONE_LIB_INPUT: &str = include_str!(r"inputs/one_library.txt");
#[cfg(test)]
static ONE_BINARY_INPUT: &str = include_str!(r"inputs/one_binary.txt");
#[cfg(test)]
static MULTIPLE_CRATES_INPUT: &str = include_str!(r"inputs/multiple_crates.txt");

/*
#[cfg(test)]
mod eat_to_next_linefeed_tests {
    use super::*;

    #[test]
    fn empty_input() {
        let result = eat_to_next_linefeed("");
        assert_eq!(result, EatenData::end(""));
    }

    #[test]
    fn linefeed_alone() {
        let result = eat_to_next_linefeed("\n");
        assert_eq!(result, EatenData::end("\n"));
    }

    #[test]
    fn linefeed_with_more() {
        let result = eat_to_next_linefeed("\nabc");
        assert_eq!(result, EatenData::more("\n", "abc"));
    }

    #[test]
    fn word_alone() {
        let result = eat_to_next_linefeed("abc ");
        assert_eq!(result, EatenData::end("abc "));
    }

    #[test]
    fn line_alone() {
        let result = eat_to_next_linefeed("abc \r\n");
        assert_eq!(result, EatenData::end("abc \r\n"));
    }

    #[test]
    fn line_alone_with_more() {
        let result = eat_to_next_linefeed("abc \r\ndef");
        assert_eq!(result, EatenData::more("abc \r\n", "def"));
    }
}
*/

/*
#[cfg(test)]
mod crate_name_tests {
    use super::*;

    #[test]
    fn new_for_empty_full_name() {
        let result = CrateName::new("").unwrap_err();
        assert_eq!(result, ParseError::MalformedCrateName("".into()));
    }

    #[test]
    fn new_for_full_name_with_no_guid() {
        let result = CrateName::new("/long/path").unwrap_err();
        assert_eq!(result, ParseError::MalformedCrateName("/long/path".into()));
    }

    #[test]
    fn new_for_full_name_with_valid_guid() {
        let result = CrateName::new("/long/path-9bdf7ee7378a8684").unwrap();
        assert_eq!(result.full_name, "/long/path-9bdf7ee7378a8684");
        assert_eq!(result.pretty_name, "/long/path");
        assert_eq!(result.uuid, "9bdf7ee7378a8684");
    }
}
*/

/*
#[cfg(test)]
mod parse_crate_name_tests {
    use super::*;

    #[test]
    fn empty_input() {
        let result = parse_crate_name("").unwrap_err();
        assert_eq!(result, ParseError::eof(""));
    }

    #[test]
    fn non_matching_input() {
        let result = parse_crate_name("abc").unwrap_err();
        assert_eq!(result, ParseError::eof("abc"));
    }

    #[test]
    fn not_enough_input() {
        let result = parse_crate_name("Running /home/foo/blah-9bdf7ee7378a8684").unwrap_err();
        assert_eq!(result, ParseError::eof("/home/foo/blah-9bdf7ee7378a8684"));
    }

    #[test]
    fn enough_input() {
        let result =
            parse_crate_name("Running /home/foo/blah-9bdf7ee7378a8684\nsome more").unwrap();
        assert_eq!(
            result,
            MoreData::new("/home/foo/blah-9bdf7ee7378a8684\n", "some more")
        );
    }

    #[test]
    fn untrimmed_start() {
        let result = parse_crate_name("  Running /home/foo/blah-9bdf7ee7378a8684").unwrap_err();
        assert_eq!(
            result,
            ParseError::eof("  Running /home/foo/blah-9bdf7ee7378a8684")
        );
    }
}
*/

/*
#[cfg(test)]
mod parse_crate_tests {
    use super::*;

    #[test]
    fn one_lib() {
        match parse_crate(ONE_LIB_INPUT).unwrap() {
            ParsedData::Done => panic!("Bad parse"),
            ParsedData::CrateTest { tests, remainder } => {
                assert_eq!(
                    tests.crate_name,
                    "/home/phil/repos/rtest/target/debug/deps/example_lib_tests-9bdf7ee7378a8684"
                );
            }
        }
    }
}
*/

/*
#[cfg(test)]
mod utility_function_tests {
    use super::*;

    #[test]
    fn is_valid_uuid_for_empty_string() {
        let result = is_valid_uuid("").unwrap_err();
        assert_eq!(result, ParseError::MalformedUuid("".into()));
    }

    #[test]
    fn is_valid_uuid_for_valid_uuid_lowercase() {
        let result = is_valid_uuid("9bdf7ee7378a8684").unwrap();
        assert_eq!(result, "9bdf7ee7378a8684");
    }

    #[test]
    fn is_valid_uuid_for_valid_uuid_uppercase() {
        let result = is_valid_uuid("9BDF7EE7378A8684").unwrap();
        assert_eq!(result, "9BDF7EE7378A8684");
    }

    #[test]
    fn is_valid_uuid_for_start_padded_uuid() {
        let result = is_valid_uuid("-9bdf7ee7378a8684").unwrap_err();
        assert_eq!(
            result,
            ParseError::MalformedUuid("-9bdf7ee7378a8684".into())
        );
    }

    #[test]
    fn is_valid_uuid_for_end_padded_uuid() {
        let result = is_valid_uuid("9bdf7ee7378a8684\n").unwrap_err();
        assert_eq!(
            result,
            ParseError::MalformedUuid("9bdf7ee7378a8684\n".into())
        );
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
}
*/