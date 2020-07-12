/// Parses the output of `cargo test -- --list` and returns the result.
/// There will be one entry in the result for each crate that was
/// parsed.
pub fn parse_test_list(data: &[u8]) -> Vec<CrateTests> {
    vec![]
}

pub struct CrateTests {
    pub crate_name: String,
    pub unit_tests: Vec<String>,
    pub doc_tests: Vec<DocTest>
}

pub struct DocTest {
    pub name: String,
    pub line: usize,
    pub file: String,
}


pub enum TestType {
    Unit,
    Doc,
}

/*
- Recognise a unit test section
    - Starts ^[ws]Running CRATE_NAME-GUID$
    - Ends ^N tests, M benchmarks$
    - Recognise a unit test name
        - ^[MODULEPATH::TESTNAME]: test$

        Recognise a doc section
    - Starts ^[ws]Doc-tests CRATE_NAME$
    - Ends ^N tests, M benchmarks$
    - May be empty ("0 tests, 0 benchmarks")
    - Recognise a doc test name
        - ^[FILENAME].rs - TESTNAME (line N): test$


nom
    - skip over irrelevant material
    - skip over ws
*/

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn running_parser_works() {
    }
}
