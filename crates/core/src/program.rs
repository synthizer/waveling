use anyhow::Result;
use petgraph::{prelude::*, stable_graph::DefaultIx};

use crate::{Edge, Node, Op, PrimitiveType, SourceLoc, VectorDescriptor};

/// The type of the graph containing this program's operations.
///
/// This is a directed graph where edges point from their outputs to their inputs, e.g. `read input -> some math ->
/// write output`.
pub type OperationGraph = DiGraph<Node, Edge>;
pub type OperationGraphNode = NodeIndex<DefaultIx>;

/// The program represents a graph defining an audio effect, and its surrounding environment.
///
/// The fields of this struct are public due to our desire to split things into different crates.  Rust borrowing
/// limitations require this for field splitting.
#[derive(Debug)]
pub struct Program {
    pub inputs: Vec<VectorDescriptor>,
    pub outputs: Vec<VectorDescriptor>,
    pub properties: Vec<PrimitiveType>,
    pub graph: OperationGraph,

    /// The start node, e.g. [Op::Start].
    ///
    /// Created on creation. A second should never be added.
    pub start_node: OperationGraphNode,

    /// The final node, e.g. [Op::Final].
    ///
    /// Added on creation.  A second should never be created.
    pub final_node: OperationGraphNode,
}

impl Program {
    pub fn new() -> Self {
        let mut graph: OperationGraph = Default::default();

        let start_node = graph.add_node(Node {
            op: Op::Start,
            shape: None,
            source_loc: None,
        });

        let final_node = graph.add_node(Node {
            op: Op::Start,
            shape: None,
            source_loc: None,
        });

        Program {
            inputs: vec![],
            outputs: vec![],
            properties: vec![],
            graph,
            start_node,
            final_node,
        }
    }

    /// Add an input, which must be a nonzero-width vector of a primitive type.
    ///
    /// Return the index to this input.
    pub fn add_input(&mut self, primitive: PrimitiveType, width: u64) -> Result<usize> {
        if width == 0 {
            anyhow::bail!("Inputs must not be of zero width");
        }

        self.inputs.push(VectorDescriptor { primitive, width });
        Ok(self.inputs.len() - 1)
    }

    /// Add an output.
    ///
    /// Outputs must be nonzero-width vectors of a primitive type.
    ///
    /// Returns the index to the new output.
    pub fn add_output(&mut self, primitive: PrimitiveType, width: u64) -> Result<usize> {
        if width == 0 {
            anyhow::bail!("Outputs must not be of zero width");
        }

        self.outputs.push(VectorDescriptor { primitive, width });
        Ok(self.inputs.len() - 1)
    }

    /// Add a property, a scalar input to the program.
    ///
    /// Return the index of the new property.
    pub fn add_property(&mut self, primitive: PrimitiveType) -> Result<usize> {
        self.properties.push(primitive);
        Ok(self.properties.len() - 1)
    }

    /// Connect a node to the given input of another node.
    ///
    /// All nodes currently have one output only.
    pub fn connect(
        &mut self,
        from_node: OperationGraphNode,
        to_node: OperationGraphNode,
        to_input: u16,
        source_loc: Option<SourceLoc>,
    ) -> Result<()> {
        let edge = Edge {
            input: to_input,
            source_loc,
        };

        // petgraph doesn't validate these, so we have to.
        if self.graph.node_weight(from_node).is_none() {
            anyhow::bail!("Graph doesn't contain the source node");
        }

        if self.graph.node_weight(to_node).is_none() {
            anyhow::bail!("Graph doesn't contain the destination node");
        }

        // We do actually want to allow multiple edges here, since the output could in theory connect to the same
        // input.
        self.graph.add_edge(from_node, to_node, edge);
        Ok(())
    }
}

impl Default for Program {
    fn default() -> Self {
        Self::new()
    }
}
