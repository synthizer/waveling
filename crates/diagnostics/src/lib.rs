/// A span in source text, which we use to track for errors.
///
/// We don't use the one from Pest, because the one from Pest holds a reference to the source text and that infests
/// everything everywhere with a lifetime parameter.
#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct Span {
    pub start: usize,
    pub end: usize,

    pub start_line: usize,
    pub start_line_col: usize,
    pub end_line: usize,
    pub end_line_col: usize,
}

/// A compilation error.
#[derive(Debug, thiserror::Error)]
pub struct CompilationError {
    reason: String,
    span: Option<Span>,
}

impl CompilationError {
    pub fn new(span: Option<Span>, reason: impl AsRef<str>) -> CompilationError {
        CompilationError {
            span,
            reason: reason.as_ref().to_string(),
        }
    }

    pub fn get_reason(&self) -> &str {
        &self.reason
    }

    pub fn get_span(&self) -> Option<&Span> {
        self.span.as_ref()
    }
}

impl std::fmt::Display for CompilationError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match self.span {
            Some(s) => write!(
                formatter,
                "At {}:{}: {}",
                s.start_line, s.start_line_col, self.reason
            )?,
            None => write!(formatter, "At unknown location: {}", self.reason)?,
        }

        Ok(())
    }
}

impl<'i> From<&pest::Span<'i>> for Span {
    fn from(input: &pest::Span<'i>) -> Span {
        let (start_line, start_line_col) = input.start_pos().line_col();
        let (end_line, end_line_col) = input.end_pos().line_col();

        Span {
            start: input.start(),
            end: input.end(),
            start_line,
            start_line_col,
            end_line,
            end_line_col,
        }
    }
}

impl<'i> From<pest::Span<'i>> for Span {
    fn from(input: pest::Span<'i>) -> Span {
        (&input).into()
    }
}
