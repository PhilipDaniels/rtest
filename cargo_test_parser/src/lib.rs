mod crate_name;
mod doc_test;
mod parse_context;
mod parse_error;
mod utils;

pub use parse_error::ParseError;
pub use crate_name::CrateName;
use doc_test::DocTest;
use parse_context::ParseContext;
use utils::parse_leading_usize;

/// Parses the output of `cargo test -- --list` and returns the result.
/// There will be one entry in the result vector for each crate that was
/// parsed. Within each crate, the tests and doc tests are listed
/// separately. Note: benchmarks are currently not supported because
/// they are not available in stable rust without 3rd party support,
/// and there are multiple ways of doing that.
///
/// # Performance
/// The parsing does not allocate any Strings, it only borrows references
/// to the input `data`. It will allocate some vectors.
pub fn parse_test_list(data: &str) -> Result<Vec<Tests>, ParseError> {
    const RUNNING_PREFIX: &str = "Running ";
    const DOC_TEST_PREFIX: &str = "Doc-tests ";

    let mut tests = Vec::new();
    let mut ctx = ParseContext::new(data);

    while let Some(line) = ctx.next() {
        let line = line.trim();

        if line.starts_with(RUNNING_PREFIX) {
            // Ok, we found a standard test listing.
            let line = line.trim_start_matches(RUNNING_PREFIX);
            let crate_name = CrateName::parse(line, &ctx)?;
            let mut crate_tests = Tests {
                crate_name,
                tests: Vec::new(),
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
                // This indicates we improperly ran over into another section.
                if line.starts_with(RUNNING_PREFIX) || line.starts_with(DOC_TEST_PREFIX) {
                    return Err(ParseError::section_overrun(&ctx));
                }

                if let Some((num_tests, _num_benches)) = parse_test_summary_count(line) {
                    // Check that we extracted the same number of items as
                    // the summary line claims there are.
                    if crate_tests.tests.len() != num_tests {
                        return Err(ParseError::unit_test_miscount(&ctx, crate_tests.tests.len()));
                    }
                    // TODO: Check benchmarks here.

                    break;
                }

                if let Some(test_name) = parse_unit_test(line) {
                    crate_tests.tests.push(test_name);
                }
            }

            tests.push(crate_tests)
        } else if line.starts_with(DOC_TEST_PREFIX) {
            // Ok we found a set of doc tests. The crate for these has *probably* already
            // been seen, so we try to attach to the one already in the `tests` vector
            // or create a new Tests if there isn't one already.
            // The line is of the form "  Doc-tests some_crate_name"
            let line = line.trim_start_matches(DOC_TEST_PREFIX);
            let crate_name = line.trim();

            if tests
                .iter_mut()
                .find(|ct| ct.crate_name.basename == crate_name)
                .is_none()
            {
                dbg!(line);
                let crate_tests = Tests {
                    crate_name: CrateName::parse(line, &ctx).unwrap(),
                    tests: Vec::new(),
                    doc_tests: Vec::new(),
                };
                tests.push(crate_tests);
            }

            let crate_tests = tests
                .iter_mut()
                .find(|ct| ct.crate_name.basename == crate_name)
                .unwrap();

            // Now attach all the doc tests to `crate_tests`.
            while let Some(line) = ctx.next() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                // This indicates we improperly ran over into another section.
                if line.starts_with(RUNNING_PREFIX) || line.starts_with(DOC_TEST_PREFIX) {
                    return Err(ParseError::section_overrun(&ctx));
                }

                if let Some((num_tests, _num_benches)) = parse_test_summary_count(line) {
                    // Check that we extracted the same number of items as
                    // the summary line claims there are.
                    if crate_tests.doc_tests.len() != num_tests {
                        return Err(ParseError::unit_test_miscount(&ctx, crate_tests.doc_tests.len()));
                    }
                    // TODO: Check benchmarks here.

                    break;
                }

                let doc_test = DocTest::parse(line, &ctx)?;
                crate_tests.doc_tests.push(doc_test);
            }
        }
    }

    Ok(tests)
}

/// Represents the set of unit tests (normal tests or benchmarks)
/// in a single crate.
#[derive(Debug, Clone)]
pub struct Tests<'a> {
    pub crate_name: CrateName<'a>,
    pub tests: Vec<&'a str>,
    pub doc_tests: Vec<DocTest<'a>>,
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

/// Parse a line of the form "4 tests, 2 benchmarks", returning the two counts
/// if the line matches this form, `None` otherwise.
fn parse_test_summary_count(line: &str) -> Option<(usize, usize)> {
    let mut parts = line.splitn(2, ", ");
    let p1 = parts.next();
    let p2 = parts.next();

    match (p1, p2) {
        (Some(s1), Some(s2)) => {
            if s1.ends_with(" tests") || s1.ends_with("1 test") {
                // If we fail to parse an int from the beginning of the string,
                // just assume this is a non-compliant line and return None.
                let num_tests = parse_leading_usize(s1)?;

                if s2.ends_with(" benchmarks") || s2.ends_with("1 benchmark") {
                    let num_benchmarks = parse_leading_usize(s2)?;
                    return Some((num_tests, num_benchmarks));
                }
            }

            None
        }
        _ => None,
    }
}

#[cfg(test)]
static ONE_LIB_INPUT: &str = include_str!(r"inputs/one_library.txt");
#[cfg(test)]
static ONE_BINARY_INPUT: &str = include_str!(r"inputs/one_binary.txt");
#[cfg(test)]
static MULTIPLE_CRATES_INPUT: &str = include_str!(r"inputs/multiple_crates.txt");

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
        assert_eq!(tests[0].tests.len(), 2);
        assert_eq!(tests[0].tests[0], "a::b::c");
        assert_eq!(tests[0].tests[1], "d::e::f");
    }

    #[test]
    fn parse_test_list_with_unit_test_miscount() {
        let input = "  Running /abc-9bdf7ee7378a8684
d::e::f: test

2 tests, 0 benchmarks";
        let tests = parse_test_list(input).unwrap_err();
        assert_eq!(tests.kind, ParseErrorKind::UnitTestMiscount);
    }

    #[ignore = "We don't support benchmarks yet"]
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
mod parse_test_list_doc_tests {
    use crate::{parse_error::ParseErrorKind, parse_test_list};

    #[test]
    fn parse_doc_tests_with_no_prior_unit_tests() {
        // I am not sure if this can happen, but the code supports it.
        let input = "   Doc-tests wibble
src/foo.rs - one_doc_test (line 999): test

1 tests, 0 benchmarks";

        let tests = parse_test_list(input).unwrap();
        assert_eq!(tests.len(), 1);
        assert_eq!(tests[0].crate_name.full_name, "wibble");
        assert_eq!(tests[0].tests.len(), 0);
        assert_eq!(tests[0].doc_tests.len(), 1);
        assert_eq!(tests[0].doc_tests[0].name, "one_doc_test");
    }

    #[test]
    fn parse_doc_tests_with_associated_unit_tests() {
        let input = "     Running target/debug/deps/example_bin_tests-b371342d81493fca
        tests::failing_logging_test: test
        tests::failing_printing_test: test
        tests::failing_test1: test
        tests::ignored_test: test
        tests::passing_logging_test: test
        tests::passing_printing_test: test
        tests::passing_printing_test2: test
        
7 tests, 0 benchmarks
             Running target/debug/deps/example_lib_tests-35c4554393436661
        tests::failing_logging_test: test
        tests::failing_printing_test: test
        tests::failing_test1: test
        tests::ignored_test: test
        tests::passing_logging_test: test
        tests::passing_printing_test: test
6 tests, 0 benchmarks

        Doc-tests example_lib_tests
        src/lib.rs - failing_doctest (line 21): test
        src/lib.rs - failing_printing_doctest (line 29): test
        src/lib.rs - passing_doctest (line 3): test
        src/lib.rs - passing_printing_doctest (line 11): test
        
4 tests, 0 benchmarks
";

        let tests = parse_test_list(input).unwrap();
        assert_eq!(tests.len(), 2);

        assert_eq!(tests[0].crate_name.full_name, "target/debug/deps/example_bin_tests-b371342d81493fca");
        assert_eq!(tests[0].tests.len(), 7);
        assert_eq!(tests[0].doc_tests.len(), 0);

        assert_eq!(tests[1].crate_name.full_name, "target/debug/deps/example_lib_tests-35c4554393436661");
        assert_eq!(tests[1].tests.len(), 6);
        assert_eq!(tests[1].doc_tests.len(), 4);
    }

    #[test]
    fn parse_unexpected_section() {
        let input = "     Running target/debug/deps/example_bin_tests-b371342d81493fca
        tests::failing_logging_test: test
        tests::failing_printing_test: test
        tests::failing_test1: test
        tests::ignored_test: test
        tests::passing_logging_test: test
        tests::passing_printing_test: test
        
6 tests, 0 benchmarks
             Running target/debug/deps/example_lib_tests-35c4554393436661
        tests::failing_logging_test: test
        tests::failing_printing_test: test
        tests::failing_test1: test
        tests::ignored_test: test
        tests::passing_logging_test: test
        tests::passing_printing_test: test

        Doc-tests example_lib_tests
        src/lib.rs - failing_doctest (line 21): test
        src/lib.rs - failing_printing_doctest (line 29): test
        src/lib.rs - passing_doctest (line 3): test
        src/lib.rs - passing_printing_doctest (line 11): test
        
4 tests, 0 benchmarks
";

        let tests = parse_test_list(input).unwrap_err();
        assert_eq!(tests.kind, ParseErrorKind::SectionOverrun, "Due to missing summary count line half-way down the input");
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
        let (a, b) = parse_test_summary_count("0 tests, 0 benchmarks").unwrap();
        assert_eq!(a, 0);
        assert_eq!(b, 0);

        let (a, b) = parse_test_summary_count("2 tests, 3 benchmarks").unwrap();
        assert_eq!(a, 2);
        assert_eq!(b, 3);

        // Note grammar change.
        let (a, b) = parse_test_summary_count("1 test, 3 benchmarks").unwrap();
        assert_eq!(a, 1);
        assert_eq!(b, 3);

        let (a, b) = parse_test_summary_count("2 tests, 1 benchmark").unwrap();
        assert_eq!(a, 2);
        assert_eq!(b, 1);
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
