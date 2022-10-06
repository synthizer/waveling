/// Kinds of operation associated with a node.
#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Pow,

    /// Read the given input.
    ReadInput(usize),

    /// Write the given output.
    WriteOutput(usize),

    /// The synthetic start node is used to have a single entry node, rather than n entry nodes.
    ///
    /// Doesn't carry data.
    Start,

    /// The final node is used to have a single point at which the program ends.
    ///
    /// Doesn't care about the input types and outputs nothing.
    ///
    /// This gives us a place to hook side-effecting operations which are not related to outputs, for example writing
    /// states.
    Final,
}
