use std::collections::HashSet;

use anyhow::Result;
use petgraph::{prelude::*, stable_graph::DefaultIx};

use crate::*;

/// The type of the graph containing this program's operations.
///
/// This is a directed graph where edges point from their outputs to their inputs, e.g. `read input -> some math ->
/// write output`.
pub type OperationGraph = StableDiGraph<Node, Edge>;
pub type OperationGraphNode = NodeIndex<DefaultIx>;
pub type OperationGraphEdgeRef<'a> = petgraph::stable_graph::EdgeReference<'a, Edge>;
pub type OperationGraphEdgeIndex = petgraph::graph::EdgeIndex;

/// The program represents a graph defining an audio effect, and its surrounding environment.
///
/// The fields of this struct are public due to our desire to split things into different crates.  Rust borrowing
/// limitations require this for field splitting.
#[derive(Debug)]
pub struct Program {
    pub inputs: Vec<VectorDescriptor>,
    pub outputs: Vec<VectorDescriptor>,
    pub properties: Vec<PrimitiveType>,
    pub states: Vec<State>,
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

macro_rules! decl_binop_method {
    ($name: ident, $op: ident) => {
        pub fn $name(&mut self, source_loc: Option<SourceLoc>) -> Result<OperationGraphNode> {
            Ok(self.op_node(Op::BinOp(BinOp::$op), source_loc))
        }
    };
}

macro_rules! decl_simple_op_method {
    ($name: ident, $op: ident) => {
        pub fn $name(&mut self, source_loc: Option<SourceLoc>) -> Result<OperationGraphNode> {
            Ok(self.op_node(Op::$op, source_loc))
        }
    };
}

impl Program {
    pub fn new() -> Self {
        let mut graph: OperationGraph = Default::default();

        let start_node = graph.add_node(Node {
            op: Op::Start,

            source_loc: None,
        });

        let final_node = graph.add_node(Node {
            op: Op::Final,
            source_loc: None,
        });

        Program {
            inputs: vec![],
            outputs: vec![],
            properties: vec![],
            states: vec![],
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
        to_input: usize,
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

        // We do actually want to allow multiple edges here, since the input it's connecting to has to be part of the
        // edge.  But we don't want two edges to the same input, so we validate that manually.
        let mut seen_incoming = HashSet::new();

        for i in self.graph.edges_directed(to_node, Direction::Incoming) {
            seen_incoming.insert((i.source(), i.weight().input));
        }

        if seen_incoming.contains(&(from_node, to_input)) {
            anyhow::bail!(
                "Duplicate connections from a source to a target for the same input are disallowed"
            );
        }

        self.graph.add_edge(from_node, to_node, edge);
        Ok(())
    }

    fn op_node(&mut self, op: Op, source_loc: Option<SourceLoc>) -> OperationGraphNode {
        let n = Node { op, source_loc };
        self.graph.add_node(n)
    }

    decl_binop_method!(op_add_node, Add);
    decl_binop_method!(op_sub_node, Sub);
    decl_binop_method!(op_mul_node, Mul);
    decl_binop_method!(op_div_node, Div);
    decl_simple_op_method!(op_negate_node, Negate);
    decl_simple_op_method!(op_clock_node, Clock);
    decl_simple_op_method!(op_sr_node, Sr);

    pub fn op_read_input_node(
        &mut self,
        input: usize,
        source_loc: Option<SourceLoc>,
    ) -> Result<OperationGraphNode> {
        if input > self.inputs.len() {
            anyhow::bail!(
                "Tried to read input {}n but only {} inputs are available",
                input,
                self.inputs.len()
            );
        }

        Ok(self.op_node(Op::ReadInput(input), source_loc))
    }

    pub fn op_read_property_node(
        &mut self,
        property: usize,
        source_loc: Option<SourceLoc>,
    ) -> Result<OperationGraphNode> {
        if property > self.properties.len() {
            anyhow::bail!(
                "Attempt to read property {} but only {} properties are available",
                property,
                self.properties.len()
            );
        }

        Ok(self.op_node(Op::ReadProperty(property), source_loc))
    }

    pub fn op_write_output_node(
        &mut self,
        output: usize,
        source_loc: Option<SourceLoc>,
    ) -> Result<OperationGraphNode> {
        if output > self.outputs.len() {
            anyhow::bail!(
                "Attempt to read output {} buyt only {} outputs are available",
                output,
                self.outputs.len()
            );
        }

        Ok(self.op_node(Op::WriteOutput(output), source_loc))
    }

    pub fn op_cast_node(
        &mut self,
        to_ty: PrimitiveType,
        source_loc: Option<SourceLoc>,
    ) -> Result<OperationGraphNode> {
        Ok(self.op_node(Op::Cast(to_ty), source_loc))
    }

    pub fn op_constant_node(
        &mut self,
        constant: Constant,
        source_loc: Option<SourceLoc>,
    ) -> Result<OperationGraphNode> {
        Ok(self.op_node(Op::Constant(constant), source_loc))
    }

    /// Get a cloned source location for a node.
    ///
    /// Used by the error building machinery.
    pub fn cloned_source_loc(&self, node: OperationGraphNode) -> Option<SourceLoc> {
        self.graph
            .node_weight(node)
            .expect("Should be present")
            .source_loc
            .clone()
    }

    /// get a topological sort of the graph, or return a diagnostic if there's a cycle.
    pub fn topological_sort(&self) -> SingleErrorResult<Vec<OperationGraphNode>> {
        petgraph::algo::toposort(&self.graph, None).map_err(|e| {
            let mut builder = DiagnosticBuilder::new("This graph has a cycle", None);
            builder.node_ref("This is an example node in the cycle", e.node_id());
            builder.build(self)
        })
    }

    /// Build a graphviz string for debugging purposes.
    pub fn graphviz(&self) -> String {
        petgraph::dot::Dot::new(&self.graph).to_string()
    }
}

impl Default for Program {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disallows_duplicate_edges() {
        let mut program = Program::new();
        let n1 = program.op_add_node(None).unwrap();
        let n2 = program.op_add_node(None).unwrap();
        program.connect(n1, n2, 0, None).unwrap();
        assert!(
            program.connect(n1, n2, 0, None).is_err(),
            "{}",
            program.graphviz()
        );
        // But a duplicate edge to a different input should be fine.
        program.connect(n1, n2, 1, None).unwrap();
    }
}
