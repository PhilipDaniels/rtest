use cargo_test_parser::Tests;
use log::info;
use std::{
    collections::HashMap,
    hash::Hash,
    sync::{Arc, Mutex},
};
use crate::configuration::Configuration;

/// Represents the program state (excluding the engine).
/// Basically this is the list of known tests and their state.
#[derive(Clone)]
pub struct State {
    inner: Arc<Mutex<InnerState>>,
}

pub struct InnerState {
    configuration: Configuration,
    tests: Vec<CrateTests>,
}

pub struct CrateTests {
    crate_name: CrateName,
    unit_tests: HashMap<String, UnitTest>,
}

#[derive(Debug, Clone)]
pub struct CrateName {
    /// The name with the UUID removed, for example
    /// "/home/phil/repos/rtest/target/debug/deps/example_lib_tests".
    pub name: String,

    /// The full name of the crate, including the UUID, as extracted from a 'Running' line, for
    // example:
    /// "Running /home/phil/repos/rtest/target/debug/deps/example_lib_tests-9bdf7ee7378a8684"
    pub full_name: String,

    /// The UUID part of the `full_name`.
    pub uuid: String,

    /// The base part of the `name`, for example "example_lib_tests"
    /// from "/home/phil/repos/rtest/target/debug/deps/example_lib_tests".
    pub basename: String,
}

#[derive(Debug, Clone)]
pub struct UnitTest {
    name: String,
    state: TestState,
    num_times_executed: usize,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TestState {
    NotRun,
    CompilationFailing,
    Running,
    Passed,
    Failed,
    Ignored,
}

impl InnerState {
    fn new(configuration: Configuration) -> Self {
        Self { configuration, tests: Vec::new() }
    }

    pub fn update_test_list(&mut self, test_list: &[Tests]) {
        info!(
            "Updating test list in State, passed {} crates",
            test_list.len()
        );

        for t in test_list.iter() {
            self.update_test_list_for_crate(t);
        }
    }

    fn update_test_list_for_crate(&mut self, test: &Tests) {
        let idx = match self
            .tests
            .iter()
            .position(|t| t.crate_name.full_name == test.crate_name.full_name)
        {
            Some(idx) => idx,
            None => {
                self.tests.push(CrateTests::new(&test.crate_name));
                self.tests.len() - 1
            }
        };

        // Rebuild the unit_tests collection based on the new data. Doing it this
        // way (by building a new collection) is easier than trying to adjust it
        // in-place.
        let mut crt = &mut self.tests[idx];
        let mut updated_unit_tests = HashMap::new();
        for &unit_test in &test.tests {
            let unit_test = unit_test.to_string();
            match crt.unit_tests.remove(&unit_test) {
                Some(ut) => {
                    updated_unit_tests.insert(unit_test, ut);
                }
                None => {
                    let ut = UnitTest::new(&unit_test);
                    updated_unit_tests.insert(unit_test, ut);
                }
            }
        }
        crt.unit_tests = updated_unit_tests;
        info!("There are now {} tests for crate '{}'", crt.unit_tests.len(), crt.crate_name.basename);

        // TODO: Repeat for the doc tests.
        // for &doc_test in &test.tests {}

        self.tests.sort();
    }
}

impl State {
    pub fn new(configuration: Configuration) -> Self {
        Self {
            inner: Arc::new(Mutex::new(InnerState::new(configuration))),
        }
    }

    pub fn update_test_list(&mut self, tests: &[Tests]) {
        let mut guard = self.inner.lock().unwrap();
        guard.update_test_list(tests);
    }
}

// impl Deref for State {
//     type Target = InnerState;
//     fn deref(&self) -> &Self::Target {
//         &self.inner
//     }
// }

// impl DerefMut for State {
//     fn deref_mut(&mut self) -> &mut Self::Target {
//         &mut self.inner
//     }
// }

impl CrateTests {
    fn new(name: &cargo_test_parser::CrateName<'_>) -> Self {
        Self {
            crate_name: CrateName::new(name),
            unit_tests: Default::default(),
        }
    }
}

impl CrateName {
    /// Convert to an owned form.
    fn new(name: &cargo_test_parser::CrateName<'_>) -> Self {
        Self {
            full_name: name.full_name.into(),
            basename: name.basename.into(),
            uuid: name.uuid.into(),
            name: name.name.into(),
        }
    }
}

impl UnitTest {
    fn new<S: Into<String>>(name: S) -> Self {
        Self {
            name: name.into(),
            num_times_executed: 0,
            state: TestState::NotRun,
        }
    }
}

impl PartialEq for UnitTest {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for UnitTest {}

impl Hash for UnitTest {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl PartialOrd for UnitTest {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.name.partial_cmp(&other.name)
    }
}

impl Ord for UnitTest {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(&other.name)
    }
}

impl PartialEq for CrateName {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for CrateName {}

impl Hash for CrateName {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
    }
}

impl PartialOrd for CrateName {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.name.partial_cmp(&other.name)
    }
}

impl Ord for CrateName {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(&other.name)
    }
}

impl PartialEq for CrateTests {
    fn eq(&self, other: &Self) -> bool {
        self.crate_name == other.crate_name
    }
}

impl Eq for CrateTests {}

impl Hash for CrateTests {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.crate_name.hash(state);
    }
}

impl PartialOrd for CrateTests {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.crate_name.partial_cmp(&other.crate_name)
    }
}

impl Ord for CrateTests {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.crate_name.cmp(&other.crate_name)
    }
}
