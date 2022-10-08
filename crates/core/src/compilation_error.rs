use std::borrow::Cow;
use std::fmt::{Display, Formatter};

use indenter::indented;

use crate::{OperationGraphNode, Program, SourceLoc};

/// A compilation error.
///
/// Consists of:
///
/// - A message saying what the problem is.
/// - A possible source location for the overall error, when it happens early enough that that makes sense.
/// - References to nodes with descriptions of what's wrong.
///
/// Should be created through [ErrorBuilder].
/// program is undefined behavior and in particular the node references will point at the wrong things.
#[derive(Debug)]
pub struct CompilationError {
    pub message: Cow<'static, str>,
    pub node_refs: Vec<CompilationErrorNodeRef>,
    pub source_loc: Option<SourceLoc>,
}

#[derive(Debug)]
pub struct CompilationErrorNodeRef {
    pub reason: Cow<'static, str>,
    pub node: OperationGraphNode,
    pub source_loc: Option<SourceLoc>,
}

/// Helper type for things which return a single error as a result.
pub type SingleErrorResult<T> = Result<T, CompilationError>;

/// Build [CompilationError]s.
///
/// The pattern here is `ErrorBuilder::new(message).add_ref(reason, node, ...)...build(program)`.
#[derive(Debug)]
pub struct CompilationErrorBuilder {
    error: CompilationError,
}

impl CompilationErrorBuilder {
    pub fn new(message: impl Into<Cow<'static, str>>, source_loc: Option<SourceLoc>) -> Self {
        Self {
            error: CompilationError {
                message: message.into(),
                node_refs: vec![],
                source_loc,
            },
        }
    }

    pub fn node_ref(&mut self, reason: impl Into<Cow<'static, str>>, node: OperationGraphNode) {
        self.error.node_refs.push(CompilationErrorNodeRef {
            reason: reason.into(),
            node,
            source_loc: None,
        });
    }

    pub fn build(mut self, program: &Program) -> CompilationError {
        for r in self.error.node_refs.iter_mut() {
            r.source_loc = program.cloned_source_loc(r.node);
        }

        self.error
    }
}

impl Display for CompilationError {
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
