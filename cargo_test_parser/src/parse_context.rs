/// Represents where we are in the parsing.
/// We parse by line, and it is convenient to be able to
/// peek ahead and go back a line. We handle this by
/// splitting up the entire `data` into `lines` and maintaining
/// the `current_line_number` to tell is where we have reached.
pub struct ParseContext<'a> {
    data: &'a str,
    lines: Vec<&'a str>,
    current_line_number: LineNumber,
}

enum LineNumber {
    NotStarted,
    InProgress(usize),
    Finished,
}

impl<'a> ParseContext<'a> {
    pub fn new(data: &'a str) -> Self {
        Self {
            data,
            lines: data.lines().collect(),
            current_line_number: LineNumber::NotStarted,
        }
    }

    /// Returns the current line number.
    pub fn current_line_number(&self) -> Option<usize> {
        match self.current_line_number {
            LineNumber::NotStarted => None,
            LineNumber::InProgress(idx) => Some(idx),
            LineNumber::Finished => None,
        }
    }

    /// Returns the current line. Will be the empty string if you
    /// have not yet started to iterate.
    pub fn current_line(&self) -> Option<&'a str> {
        match self.current_line_number {
            LineNumber::NotStarted => None,
            LineNumber::InProgress(idx) => Some(self.lines[idx - 1]),
            LineNumber::Finished => None,
        }
    }

    /// Reverses the iterator by one line. To get the line you are now on,
    /// call `current_line`.
    pub fn prev(&mut self) {
        self.current_line_number = match self.current_line_number {
            LineNumber::NotStarted => LineNumber::NotStarted,
            LineNumber::InProgress(idx) if idx == 1 => LineNumber::NotStarted,
            LineNumber::InProgress(idx) => LineNumber::InProgress(idx - 1),
            LineNumber::Finished => LineNumber::Finished,
        };
    }

    /// Peeks at the next line.
    pub fn peek(&mut self) -> Option<&'a str> {
        if self.lines.is_empty() {
            return None;
        }

        match self.current_line_number {
            LineNumber::NotStarted => Some(self.lines[0]),
            LineNumber::InProgress(idx) if idx == self.lines.len() => None,
            LineNumber::InProgress(idx) => Some(self.lines[idx]),
            LineNumber::Finished => None,
        }
    }
}

impl<'a> Iterator for ParseContext<'a> {
    type Item = &'a str;

    /// Returns the next line and increments the line count.
    fn next(&mut self) -> Option<Self::Item> {
        if self.lines.is_empty() {
            return None;
        }

        match self.current_line_number {
            LineNumber::NotStarted => {
                self.current_line_number = LineNumber::InProgress(1);
                Some(&self.lines[0])
            }
            LineNumber::InProgress(idx) if idx == self.lines.len() => {
                self.current_line_number = LineNumber::Finished;
                None
            }
            LineNumber::InProgress(idx) => {
                self.current_line_number = LineNumber::InProgress(idx + 1);
                Some(&self.lines[idx])
            }
            LineNumber::Finished => None,
        }
    }
}

#[cfg(test)]
mod parse_context_tests {
    use super::*;

    #[test]
    fn new_for_empty_data() {
        let mut pc = ParseContext::new("");
        assert_eq!(pc.current_line_number(), None);
        assert_eq!(pc.current_line(), None);

        let peeked_line = pc.peek();
        assert_eq!(peeked_line, None, "Peeking an empty ctx is ok");
        assert_eq!(pc.current_line_number(), None);
        assert_eq!(pc.current_line(), None);

        let line = pc.next();
        assert_eq!(line, None, "Calling next on an empty ctx is ok");
        assert_eq!(pc.current_line_number(), None);
        assert_eq!(pc.current_line(), None);
    }

    #[test]
    fn next_works() {
        let mut pc = ParseContext::new("abc\r\ndef");
        assert_eq!(pc.current_line_number(), None);
        assert_eq!(pc.current_line(), None);

        let line = pc.next();
        assert_eq!(
            pc.current_line_number(),
            Some(1),
            "Lines are counted from 1..len"
        );
        assert_eq!(pc.current_line(), Some("abc"));
        assert_eq!(line, Some("abc"));

        let line = pc.next();
        assert_eq!(pc.current_line_number(), Some(2));
        assert_eq!(pc.current_line(), Some("def"));
        assert_eq!(line, Some("def"));

        let line = pc.next();
        assert_eq!(pc.current_line_number(), None);
        assert_eq!(pc.current_line(), None);
        assert_eq!(line, None);
    }

    #[test]
    fn peek_works() {
        let mut pc = ParseContext::new("abc\r\ndef");
        assert_eq!(pc.current_line_number(), None);
        assert_eq!(pc.current_line(), None);

        let peeked_line = pc.peek();
        assert_eq!(
            peeked_line,
            Some("abc"),
            "Peeking when not yet started is ok"
        );
        assert_eq!(
            pc.current_line_number(),
            None,
            "Peeking does not change the next line"
        );
        assert_eq!(pc.current_line(), None);

        let peeked_line = pc.peek();
        assert_eq!(
            peeked_line,
            Some("abc"),
            "Peeking twice does not move us on"
        );
        assert_eq!(pc.current_line_number(), None);
        assert_eq!(pc.current_line(), None);

        let line = pc.next();
        assert_eq!(pc.current_line_number(), Some(1));
        assert_eq!(pc.current_line(), Some("abc"));
        assert_eq!(line, Some("abc"));

        let peeked_line = pc.peek();
        assert_eq!(peeked_line, Some("def"));
        assert_eq!(pc.current_line_number(), Some(1));
        assert_eq!(pc.current_line(), Some("abc"));
    }

    #[test]
    fn prev_works() {
        let mut pc = ParseContext::new("abc\r\ndef");
        pc.next();
        let line = pc.next();

        // We should be on the last line now.
        assert_eq!(line, Some("def"));
        assert_eq!(pc.current_line_number(), Some(2));
        assert_eq!(pc.current_line(), Some("def"));

        // Then the first line.
        pc.prev();
        assert_eq!(pc.current_line_number(), Some(1));
        assert_eq!(pc.current_line(), Some("abc"));

        // Then the beginning again.
        pc.prev();
        assert_eq!(pc.current_line_number(), None);
        assert_eq!(pc.current_line(), None);
    }
}
