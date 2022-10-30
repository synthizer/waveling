use std::borrow::Cow;
use std::fmt::{Display, Formatter};

use indenter::indented;

use crate::{OperationGraphNode, Program, SourceLoc};

/// A compilation diagnostic.
///
/// Consists of:
///
/// - A message saying what the problem is.
/// - A possible source location for the overall error, when it happens early enough that that makes sense.
/// - References to nodes with descriptions of what's wrong.
///
/// Should be created through [DiagnosticBuilder].
#[derive(Debug)]
pub struct Diagnostic {
    pub message: Cow<'static, str>,
    pub node_refs: Vec<DiagnosticNodeRef>,
    pub source_loc: Option<SourceLoc>,
}

/// A reference to a node involved in a diagnostic.
#[derive(Debug)]
pub struct DiagnosticNodeRef {
    pub reason: Cow<'static, str>,
    pub node: OperationGraphNode,
    pub source_loc: Option<SourceLoc>,
}

/// Helper type for things which return a single error as a result.
pub type SingleErrorResult<T> = Result<T, Diagnostic>;

/// Build [CompilationDiagnostic]s.
///
/// The pattern here is `ErrorBuilder::new(message).add_ref(reason, node, ...)...build(program)`.
#[derive(Debug)]
pub struct DiagnosticBuilder {
    diagnostic: Diagnostic,
}

/// A collection of diagnostics, for eventual display to the user.
#[derive(Debug, Default)]
pub struct DiagnosticCollection {
    pub errors: Vec<Diagnostic>,
}

impl DiagnosticBuilder {
    pub fn new(message: impl Into<Cow<'static, str>>, source_loc: Option<SourceLoc>) -> Self {
        Self {
            diagnostic: Diagnostic {
                message: message.into(),
                node_refs: vec![],
                source_loc,
            },
        }
    }

    pub fn node_ref(&mut self, reason: impl Into<Cow<'static, str>>, node: OperationGraphNode) {
        self.diagnostic.node_refs.push(DiagnosticNodeRef {
            reason: reason.into(),
            node,
            source_loc: None,
        });
    }

    pub fn build(mut self, program: &Program) -> Diagnostic {
        for r in self.diagnostic.node_refs.iter_mut() {
            r.source_loc = program.cloned_source_loc(r.node);
        }

        self.diagnostic
    }
}

impl Display for Diagnostic {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        use std::fmt::Write;

        write!(formatter, "Error: {}", self.message)?;
        if let Some(loc) = self.source_loc.as_ref() {
            writeln!(formatter)?;
            write!(indented(formatter).ind(2), "{}", loc)?;
        }

        for r in self.node_refs.iter() {
            writeln!(formatter)?;
            write!(formatter, "For node {}: {}:", r.node.index(), r.reason)?;
            if let Some(loc) = r.source_loc.as_ref() {
                writeln!(formatter)?;
                writeln!(formatter, "at:")?;
                write!(indented(formatter).ind(2), "{}", loc)?;
            }
        }

        Ok(())
    }
}

impl DiagnosticCollection {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_diagnostic(&mut self, diag: Diagnostic) {
        self.errors.push(diag);
    }

    pub fn add_simple_diagnostic(
        &mut self,
        program: &Program,
        message: impl Into<Cow<'static, str>>,
        source_loc: Option<SourceLoc>,
    ) {
        let builder = DiagnosticBuilder::new(message, source_loc);
        let diag = builder.build(program);
        self.add_diagnostic(diag);
    }
}

impl Display for DiagnosticCollection {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let mut first = false;
        for e in self.errors.iter() {
            if first {
                first = false;
            } else {
                writeln!(f)?;
            }

            write!(f, "{}", e)?;
        }

        Ok(())
    }
}
