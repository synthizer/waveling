use crate::*;

#[derive(Debug)]
pub struct Node {
    pub op: Op,

    /// When the type of the node is known this field is set.
    pub data_type: Option<DataType>,

    pub source_loc: Option<SourceLoc>,
}
