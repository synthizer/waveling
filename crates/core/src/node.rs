use std::fmt::Display;

use crate::*;

#[derive(Debug)]
pub struct Node {
    pub op: Op,

    /// When the type of the node is known this field is set.
    pub data_type: Option<DataType>,

    pub source_loc: Option<SourceLoc>,
}

impl Display for Node {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ty = match self.data_type.as_ref() {
            Some(x) => x.to_string(),
            None => "<UNKNOWN>".to_string(),
        };

        write!(f, "Node({}, of type {})", self.op, ty)
    }
}
