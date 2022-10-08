use crate::SourceLoc;

#[derive(Debug)]
pub struct Edge {
    /// Which input does this edge connect to?
    ///
    /// For example addition has two inputs, 0 and 1.
    ///
    /// If multiple edges are connected to the same input they must unify, and will implicitly sum.  This is expanded
    /// later, so that when we reach the backend, all summing is explicit addition nodes.
    pub input: u16,

    pub source_loc: Option<SourceLoc>,
}