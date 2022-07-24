use waveling_diagnostics::{CompilationError, Span};

pub fn pest_to_diagnostic(input: &pest::error::Error<crate::grammar::Rule>) -> CompilationError {
    let (start, end) = match input.location {
        pest::error::InputLocation::Pos(p) => (p, p),
        pest::error::InputLocation::Span(s) => s,
    };

    let (start_line, start_line_col, end_line, end_line_col) = match input.line_col {
        pest::error::LineColLocation::Pos((l, c)) => (l, c, l, c),
        pest::error::LineColLocation::Span(start, end) => (start.0, start.1, end.0, end.1),
    };

    let span = Span {
        start,
        end,
        start_line,
        end_line,
        start_line_col,
        end_line_col,
    };
    CompilationError::new(Some(span), &format!("{}", input))
}
