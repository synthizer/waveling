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
        let span = match input.location {
            // This hopefully doesn't come up often, because pos doesn't easily give us line numbers.
            pest::error::InputLocation::Pos(p) => crate::ast::Span {
                start: p,
                end: p,
                start_line: 0,
                start_line_col: 0,
                end_line: 0,
                end_line_col: 0,
            },
            pest::error::InputLocation::Span(s) => crate::ast::Span {
                start: s.0,
                end: s.1,
                start_line: 0,
                start_line_col: 0,
                end_line: 0,
                end_line_col: 0,
            },
        };

        Error {
            span: Some(span),
            reason: format!("{}", input),
        }
    }
}

impl From<pest::error::Error<crate::grammar::Rule>> for Error {
    fn from(input: pest::error::Error<crate::grammar::Rule>) -> Error {
        (&input).into()
    }
}
