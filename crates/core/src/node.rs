use crate::{Op, SourceLoc, VectorDescriptor};

#[derive(Debug)]
pub struct Node {
    pub op: Op,

    /// When the type of the node is known this field is set.
    ///
    /// Knowing the type can happen in two ways.  First, it can be inferred from the incoming edges. Second, it can be
    /// explicitly set by the user.
    ///
    /// The type inference algorithm will use the explicitly set types to determine the implicit ones by walking edges
    /// "forward" and applying some broadcasting/conversion rules.
    pub shape: Option<VectorDescriptor>,

    pub source_loc: Option<SourceLoc>,
}
