use crate::ast::Span;

/// An error from parsing.
///
/// Parsers accumulate as many errors as they can, and make them available to the caller for display.
#[derive(Debug, thiserror::Error)]
pub struct Error {
    reason: String,
    span: Option<Span>,
}

impl Error {
    pub(crate) fn new(span: Span, reason: impl AsRef<str>) -> Error {
        Error {
            span: Some(span),
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

impl std::fmt::Display for Error {
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

impl From<&pest::error::Error<crate::grammar::Rule>> for Error {
    fn from(input: &pest::error::Error<crate::grammar::Rule>) -> Error {
        let (start, end) = match input.location {
            pest::error::InputLocation::Pos(p) => (p, p),
            pest::error::InputLocation::Span(s) => s,
        };

        let (start_line, start_line_col, end_line, end_line_col) = match input.line_col {
            pest::error::LineColLocation::Pos((l, c)) => (l, c, l, c),
            pest::error::LineColLocation::Span(start, end) => (start.0, start.1, end.0, end.1),
        };

        Error {
            span: Some(crate::Span {
                start,
                end,
                start_line,
                end_line,
                start_line_col,
                end_line_col,
            }),
            reason: format!("{}", input),
        }
    }
}

impl From<pest::error::Error<crate::grammar::Rule>> for Error {
    fn from(input: pest::error::Error<crate::grammar::Rule>) -> Error {
        (&input).into()
    }
}
