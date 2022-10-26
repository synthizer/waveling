use std::fmt::Display;

use crate::*;

#[derive(Debug)]
pub struct Node {
    pub op: Op,

    pub source_loc: Option<SourceLoc>,
}

impl Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Node({})", self.op)
    }
}
