mod crate_name;
mod parse_context;
mod parse_error;
mod utils;

use crate_name::CrateName;
use parse_context::ParseContext;
use parse_error::ParseError;
use utils::parse_leading_usize;

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
            None => break,
        }
    }

    Ok(result)
}

/// Represents the set of tests in a single crate.
#[derive(Debug, Clone)]
pub struct CrateTestList<'a> {
    pub crate_name: CrateName<'a>,
    pub unit_tests: Vec<&'a str>,
    pub benchmarks: Vec<&'a str>,
    pub doc_tests: Vec<DocTest<'a>>,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct DocTest<'a> {
    pub name: &'a str,
    pub line: usize,
    pub file: &'a str,
}

/// Parse a single `CrateTestList` from the input.
fn parse_crate_test_list<'ctx, 'a>(
    ctx: &'ctx mut ParseContext<'a>,
) -> Result<Option<CrateTestList<'a>>, ParseError> {
    const PREFIX: &str = "Running ";

    while let Some(line) = ctx.next() {
        let line = line.trim();

        if line.starts_with(PREFIX) {
            // Ok, we found a test listing.
            let line = line.trim_start_matches(PREFIX);
            let crate_name = CrateName::parse(line, &ctx)?;
            let mut ctl = CrateTestList {
                crate_name,
                unit_tests: Vec::new(),
                benchmarks: Vec::new(),
                doc_tests: Vec::new(),
            };

            // Next we expect the unit tests, if any, to be listed.
            // This block will consist of lines of the form
            //      tests::failing_test1: test
            // and be terminated by a line of the form
            //      "6 tests, 4 benchmarks"
            while let Some(line) = ctx.next() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                if let Some((num_tests, num_benches)) = parse_test_summary_count(line) {
                    // Check that we extracted the same number of items as
                    // the summary line claims there are.
                    if ctl.unit_tests.len() != num_tests {
                        return Err(ParseError::unit_test_miscount(ctx));
                    }
                    if ctl.benchmarks.len() != num_benches {
                        return Err(ParseError::benchmark_miscount(ctx));
                    }

                    break;
                }

                if let Some(test_name) = parse_unit_test(line) {
                    ctl.unit_tests.push(test_name);
                } else if let Some(test_name) = parse_bench_test(line) {
                    ctl.benchmarks.push(test_name);
                }
            }

            return Ok(Some(ctl));
        }
    }

    Ok(None)
}

/// Parses a line of the form "tests::failing_test1: test", as occurs when the
/// unit tests are being listed. Returns the name of the test if the parse
/// succeeds, `None` otherwise.
fn parse_unit_test(line: &str) -> Option<&str> {
    let line = line.trim();

    // TODO: Not sure what the point of this trailing test is.
    // It might be where "benchmarks" are distinguished.
    if line.ends_with(": test") {
        Some(line.trim_end_matches(": test"))
    } else {
        None
    }
}

/// Parses a line of the form "tests::failing_test1: bench", as occurs when the
/// unit tests are being listed. Returns the name of the test if the parse
/// succeeds, `None` otherwise.
fn parse_bench_test(line: &str) -> Option<&str> {
    let line = line.trim();

    // TODO: Not sure what the point of this trailing test is.
    // It might be where "benchmarks" are distinguished.
    if line.ends_with(": bench") {
        Some(line.trim_end_matches(": bench"))
    } else {
        None
    }
}

fn parse_doc_test(line: &str) -> Option<&str> {
    None
}

/// Parse a line of the form "4 tests, 2 benchmarks", returning the two counts
/// if the line matches this form, `None` otherwise.
fn parse_test_summary_count(line: &str) -> Option<(usize, usize)> {
    let mut parts = line.splitn(2, ", ");
    let p1 = parts.next();
    let p2 = parts.next();

    match (p1, p2) {
        (Some(s1), Some(s2)) => {
            if s1.ends_with(" tests") {
                // If we fail to parse an int from the beginning of the string,
                // just assume this is a non-compliant line and return None.
                let num_tests = parse_leading_usize(s1)?;

                if s2.ends_with(" benchmarks") {
                    let num_benchmarks = parse_leading_usize(s2)?;
                    return Some((num_tests, num_benchmarks));
                }
            }

            None
        }
        _ => None,
    }
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

// #[derive(Debug, Clone, PartialEq, Eq)]
// struct MoreData<'a> {
//     data: &'a str,
//     remainder: &'a str,
// }

// impl<'a> MoreData<'a> {
//     /// Constructor for a `MoreData` instance.
//     fn new(data: &'a str, remainder: &'a str) -> Self {
//         Self { data, remainder }
//     }
// }

// /// Represents the result of an 'eat' operation.
// #[derive(Debug, Clone, PartialEq, Eq)]
// enum EatenData<'a> {
//     /// All the data was eaten, there is no more to follow.
//     EndOfData { data: &'a str },
//     /// Some of the data was eaten, but there is more to follow.
//     MoreData(MoreData<'a>),
// }

// impl<'a> EatenData<'a> {
//     /// Constructor for the EndOfData variant.
//     fn end(data: &'a str) -> Self {
//         Self::EndOfData { data: data }
//     }

//     /// Constructor the the MoreData variant.
//     fn more(data: &'a str, remainder: &'a str) -> Self {
//         Self::MoreData(MoreData::new(data, remainder))
//     }
// }

// #[derive(Debug, Clone)]
// pub enum ParsedData<'a> {
//     Done,
//     CrateTest {
//         tests: CrateTestList<'a>,
//         remainder: &'a str,
//     },
// }

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

/// A bunch of tests that just check that our extract-next collection sequence
/// for `CrateTestList` works. Does not check that we can extract the names
/// of the unit and doc tests themselves.
#[cfg(test)]
mod parse_test_list_simple_tests {
    use crate::parse_test_list;

    #[test]
    fn parse_test_list_for_empty_data() {
        let tests = parse_test_list("").unwrap();
        assert!(tests.is_empty());
    }

    #[test]
    fn parse_test_list_for_one_crate_without_preamble() {
        let tests =
            parse_test_list("  Running /abc-9bdf7ee7378a8684\n0 tests, 0 benchmarks").unwrap();
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].crate_name.full_name, "/abc-9bdf7ee7378a8684");
    }

    #[test]
    fn parse_test_list_for_one_crate_with_preamble() {
        let tests = parse_test_list("  Finished test  [unoptimized + debuginfo] target(s) in 0.05s\n  Running /abc-9bdf7ee7378a8684\n0 tests, 0 benchmarks").unwrap();
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].crate_name.full_name, "/abc-9bdf7ee7378a8684");
    }

    #[test]
    fn parse_test_list_for_two_crates_with_no_bodies() {
        let tests = parse_test_list("  Running /abc-9bdf7ee7378a8684\n0 tests, 0 benchmarks\n  Running /def-0490fca25dc32581\n0 tests, 0 benchmarks").unwrap();
        assert_eq!(tests.len(), 2);
        assert_eq!(tests[0].crate_name.full_name, "/abc-9bdf7ee7378a8684");
        assert_eq!(tests[1].crate_name.full_name, "/def-0490fca25dc32581");
    }
}

/// A bunch of tests that check we are extracting unit tests correctly.
#[cfg(test)]
mod parse_test_list_unit_tests {
    use crate::{parse_error::ParseErrorKind, parse_test_list};

    #[test]
    fn parse_test_list_for_one_crate_without_preamble() {
        let input = "  Running /abc-9bdf7ee7378a8684
a::b::c: test
d::e::f: test

2 tests, 0 benchmarks";
        let tests = parse_test_list(input).unwrap();
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].crate_name.full_name, "/abc-9bdf7ee7378a8684");
        assert_eq!(tests[0].unit_tests.len(), 2);
        assert_eq!(tests[0].unit_tests[0], "a::b::c");
        assert_eq!(tests[0].unit_tests[1], "d::e::f");
    }

    #[test]
    fn parse_test_list_with_unit_test_miscount() {
        let input = "  Running /abc-9bdf7ee7378a8684
d::e::f: test

2 tests, 0 benchmarks";
        let tests = parse_test_list(input).unwrap_err();
        assert_eq!(tests.kind, ParseErrorKind::UnitTestMiscount);
    }

    #[test]
    fn parse_test_list_with_benchmark_miscount() {
        let input = "  Running /abc-9bdf7ee7378a8684
d::e::f: bench

0 tests, 2 benchmarks";
        let tests = parse_test_list(input).unwrap_err();
        assert_eq!(tests.kind, ParseErrorKind::BenchmarkMiscount);
    }
}

#[cfg(test)]
mod parse_test_summary_count_tests {
    use crate::parse_test_summary_count;

    #[test]
    fn parse_for_empty_data() {
        assert!(parse_test_summary_count("").is_none());
    }

    #[test]
    fn parse_for_truncated_data() {
        assert!(parse_test_summary_count("0 tests").is_none());
        assert!(parse_test_summary_count("0 tests,").is_none());
        assert!(parse_test_summary_count("0 tests, 2").is_none());
    }

    #[test]
    fn parse_for_good_data() {
        let (a, b) = parse_test_summary_count("1 tests, 2 benchmarks").unwrap();
        assert_eq!(a, 1);
        assert_eq!(b, 2);
    }
}

#[cfg(test)]
mod parse_unit_test_tests {
    use crate::parse_unit_test;

    #[test]
    fn parse_for_empty_data() {
        assert!(parse_unit_test("").is_none());
    }

    #[test]
    fn parse_for_truncated_data() {
        assert!(parse_unit_test(" a ").is_none());
        assert!(parse_unit_test("a::b::c").is_none());
        assert!(parse_unit_test("a::b::c:test").is_none());
        assert!(parse_unit_test("a::b::c: tes").is_none());
    }

    #[test]
    fn parse_for_good_data() {
        assert_eq!(parse_unit_test("a: test"), Some("a"));
        assert_eq!(parse_unit_test(" a::b::c: test "), Some("a::b::c"));
    }
}
