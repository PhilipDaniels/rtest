use std::str::Lines;

pub struct ParseContext<'a> {
    data: &'a str,
    lines: Lines<'a>,
    current_line_number: usize,
}

impl<'a> ParseContext<'a> {
    fn new(data: &'a str) -> Self {
        Self {
            data,
            lines: data.lines(),
            current_line_number: 0,
        }
    }
}

impl<'a> Iterator for ParseContext<'a> {
    type Item = &'a str;

    /// Returns the next line and increments the line count.
    fn next(&mut self) -> Option<Self::Item> {
        match self.lines.next() {
            Some(line) => {
                self.current_line_number += 1;
                Some(line)
            }
            None => None
        }
    }
}

#[cfg(test)]
mod parse_context_tests {
    use super::*;

    #[test]
    fn new_for_empty_data() {
        let pc = ParseContext::new("");
    }

    #[test]
    fn next_works() {
        let mut pc = ParseContext::new("abc\r\ndef");

        let line = pc.next();
        assert_eq!(pc.current_line_number, 1);
        assert_eq!(line, Some("abc"));

        let line = pc.next();
        assert_eq!(pc.current_line_number, 2);
        assert_eq!(line, Some("def"));

        let line = pc.next();
        assert_eq!(pc.current_line_number, 2);
        assert_eq!(line, None);
    }
}
