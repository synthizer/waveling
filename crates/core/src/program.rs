use anyhow::Result;
use petgraph::{prelude::*, stable_graph::DefaultIx};

use crate::{
    BinOp, DiagnosticBuilder, Edge, Node, Op, PrimitiveType, SingleErrorResult, SourceLoc, State,
    VectorDescriptor,
};

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
            Ok(self.op_node(Op::BinOp(BinOp::$op), None, source_loc))
        }
    };
}

macro_rules! decl_simple_op_method {
    ($name: ident, $op: ident) => {
        pub fn $name(&mut self, source_loc: Option<SourceLoc>) -> Result<OperationGraphNode> {
            Ok(self.op_node(Op::$op, None, source_loc))
        }
    };
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

    /// Add a state, a memory location consisting of some number of consecutive vectors.
    ///
    /// The width of the vector and length of the state must both be nonzero.
    ///
    /// Returns the index to the state.
    pub fn add_state(
        &mut self,
        primitive: PrimitiveType,
        width: u64,
        length: u64,
    ) -> Result<usize> {
        if width == 0 {
            anyhow::bail!("State vector widths must not be zero");
        }

        if length == 0 {
            anyhow::bail!("State lengths must not be zero");
        }

        let vd = VectorDescriptor { primitive, width };
        let st = State { length, vector: vd };
        self.states.push(st);
        Ok(self.states.len() - 1)
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

    fn op_node(
        &mut self,
        op: Op,
        shape: Option<VectorDescriptor>,
        source_loc: Option<SourceLoc>,
    ) -> OperationGraphNode {
        let n = Node {
            op,
            shape,
            source_loc,
        };
        self.graph.add_node(n)
    }

    decl_binop_method!(op_add_node, Add);
    decl_binop_method!(op_sub_node, Sub);
    decl_binop_method!(op_mul_node, Mul);
    decl_binop_method!(op_div_node, Div);
    decl_simple_op_method!(op_negate_node, Negate);
    decl_simple_op_method!(op_clock_node, Clock);

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

        Ok(self.op_node(Op::ReadInput(input), None, source_loc))
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

        Ok(self.op_node(Op::ReadProperty(property), None, source_loc))
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

        Ok(self.op_node(Op::WriteOutput(output), None, source_loc))
    }

    fn op_read_state_node_impl(
        &mut self,
        state: usize,
        source_loc: Option<SourceLoc>,
        modulus: bool,
    ) -> Result<OperationGraphNode> {
        if state >= self.states.len() {
            anyhow::bail!(
                "Only {} states available, but tried to read state {}",
                self.states.len(),
                state
            );
        }

        Ok(self.op_node(Op::ReadState { state, modulus }, None, source_loc))
    }

    fn op_write_state_node_impl(
        &mut self,
        state: usize,
        source_loc: Option<SourceLoc>,
        modulus: bool,
    ) -> Result<OperationGraphNode> {
        if state >= self.states.len() {
            anyhow::bail!(
                "Only {} states available, but tried to write state {}",
                self.states.len(),
                state
            );
        }

        Ok(self.op_node(Op::WriteState { state, modulus }, None, source_loc))
    }

    /// Read a state directly, without modulus.
    pub fn op_read_state_direct_node(
        &mut self,
        state: usize,
        source_loc: Option<SourceLoc>,
    ) -> Result<OperationGraphNode> {
        self.op_read_state_node_impl(state, source_loc, false)
    }

    /// Read a state, with modulus.
    pub fn op_read_state_mod_node(
        &mut self,
        state: usize,
        source_loc: Option<SourceLoc>,
    ) -> Result<OperationGraphNode> {
        self.op_read_state_node_impl(state, source_loc, true)
    }

    /// Write a state directly, without modulus on the location in the state.
    pub fn op_write_state_direct_node(
        &mut self,
        state: usize,
        source_loc: Option<SourceLoc>,
    ) -> Result<OperationGraphNode> {
        self.op_write_state_node_impl(state, source_loc, false)
    }

    pub fn op_write_state_mod_node(
        &mut self,
        state: usize,
        source_loc: Option<SourceLoc>,
    ) -> Result<OperationGraphNode> {
        self.op_write_state_node_impl(state, source_loc, true)
    }

    pub fn op_cast_node(
        &mut self,
        to_ty: PrimitiveType,
        source_loc: Option<SourceLoc>,
    ) -> Result<OperationGraphNode> {
        Ok(self.op_node(Op::Cast(to_ty), None, source_loc))
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
}

impl Default for Program {
    fn default() -> Self {
        Self::new()
    }
}
