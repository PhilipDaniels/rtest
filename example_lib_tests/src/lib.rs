/// A random comment. Doc tests only work in libs.
///
/// ```
/// let a = 3;
/// assert_eq(a, 3);
/// ```
fn passing_doctest() { }

/// A random comment. Doc tests only work in libs.
///
/// ```
/// println!("This is a println in passing_printing_doctest");
/// eprintln!("This is an eprintln in passing_printing_doctest");
/// let a = 3;
/// assert_eq(a, 3);
/// ```
fn passing_printing_doctest() { }

/// A random comment. Doc tests only work in libs.
///
/// ```
/// let a = 3;
/// assert_eq(a, 4);
/// ```
fn failing_doctest() { }

/// A random comment. Doc tests only work in libs.
///
/// ```
/// println!("This is a println in failing_printing_doctest");
/// eprintln!("This is an eprintln in failing_printing_doctest");
/// let a = 3;
/// assert_eq(a, 4);
/// ```
fn failing_printing_doctest() { }



#[cfg(test)]
mod tests {
    use log::info;

    fn init_logger() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    #[test]
    #[ignore]
    pub fn ignored_test() {
        println!("This is a println in ignored_test");
        assert_eq!(3, 4)
    }

    #[test]
    pub fn passing_printing_test() {
        println!("This is a println in passing_printing_test");
        eprintln!("This is an eprintln in passing_printing_test");
        assert_eq!(3, 3)
    }

    #[test]
    pub fn failing_printing_test() {
        // Note that these will be captured, but any prints after
        // the assertion will not be (the thread panics at that point).
        println!("This is a println in failing_printing_test");
        eprintln!("This is an eprintln in failing_printing_test");
        assert_eq!(3, 4);
    }

    #[test]
    pub fn passing_logging_test() {
        init_logger();
        println!("This is a println in failing_logging_test");
        info!("This is an info message in passing_logging_test");
        eprintln!("This is an eprintln in failing_logging_test");
        assert_eq!(3, 3)
    }

    #[test]
    pub fn failing_logging_test() {
        init_logger();
        println!("This is a println in failing_logging_test");
        info!("This is an info message in passing_logging_test");
        eprintln!("This is an eprintln in failing_logging_test");
        assert_eq!(3, 4)
    }

   #[test]
    pub fn failing_test1() {
        let a = 3;
        println!("This is a println in failing_test1");
        assert_eq!(a, 33)
    }
}
