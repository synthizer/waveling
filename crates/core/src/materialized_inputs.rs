use petgraph::prelude::*;
use smallvec::SmallVec;

use crate::*;

/// represents a set of inputs to a node at a higher level than raw edges.
///
/// A problem that we face is that we need to use petgraph, but some passes need to know actual structure. This type
/// materializes the inputs of a node into something that allows access by input index, though it currently leaves
/// actually getting node weights to the user (because doing otherwise would in effect require cloning expensive things,
/// for example `Op::Constant`).
#[derive(Debug, Clone)]
pub struct MaterializedInputs {
    /// The inputs.
    ///
    /// note that using the helper methods can e.g. return slices to inputs that aren't in this vec. That is, if a node
    /// has 2 inputs and only uses the first (index 0) then the helper methods account for this and pretend that the
    /// node has any number of possibly empty inputs, but raw indexing doesn't.
    pub inputs: SmallVec<[SmallVec<[MaterializedInput; 4]>; 2]>,
}

#[derive(Debug, Clone)]
pub struct MaterializedInput {
    /// The input node.
    pub source_node: OperationGraphNode,

    /// The target of the edge, which is always the node the [MaterializedInputs] was materialized with.
    pub target_node: OperationGraphNode,

    /// The id of the edge, if the user needs more info than we copy down.
    pub edge: OperationGraphEdgeIndex,
}

impl MaterializedInputs {
    /// materialize the information for a set of inputs.
    ///
    /// panics if the node isn't in the graph.
    pub fn materialize(program: &Program, node: OperationGraphNode) -> MaterializedInputs {
        Self::materialize_with_filter(program, node, |_| true)
    }

    /// Materialize the inputs for a node, but only if a filter returns true.
    ///
    /// This is used for instance to filter out the start node.
    pub fn materialize_with_filter(
        program: &Program,
        node: OperationGraphNode,
        mut filter: impl FnMut(OperationGraphNode) -> bool,
    ) -> MaterializedInputs {
        program
            .graph
            .node_weight(node)
            .expect("Node should be in the graph");

        let mut ret = MaterializedInputs {
            inputs: Default::default(),
        };

        for e in program.graph.edges_directed(node, Direction::Incoming) {
            let owned_edge = e.id();
            let source_node = e.source();
            if !filter(source_node) {
                continue;
            }

            let target_node = e.target();

            // If this edge has an input greater than what's in our vec, we must extend it.
            if e.weight().input >= ret.inputs.len() {
                ret.inputs.resize(e.weight().input + 1, Default::default());
            }

            ret.inputs[e.weight().input].push(MaterializedInput {
                source_node,
                target_node,
                edge: owned_edge,
            });
        }

        ret
    }

    /// Get the materialized input for an index.
    ///
    /// Returns an emptty slice for unused inputs.
    pub fn get_input(&self, index: usize) -> &[MaterializedInput] {
        self.inputs.get(index).map(|x| &x[..]).unwrap_or(&[])
    }

    /// Get a mutable slice over the materialized input for an index.
    ///
    /// Useful for e.g. sorting.
    ///
    /// Returns an emptty slice for unused inputs.
    pub fn get_input_mut(&mut self, index: usize) -> &mut [MaterializedInput] {
        self.inputs
            .get_mut(index)
            .map(|x| &mut x[..])
            .unwrap_or(&mut [])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_materializing_inputs() {
        let mut program = Program::new();

        let input1 = program.op_add_node(None).unwrap();
        let input2 = program.op_add_node(None).unwrap();
        let input3 = program.op_add_node(None).unwrap();
        let input4 = program.op_add_node(None).unwrap();
        let no_inputs = program.op_add_node(None).unwrap();
        let both_inputs = program.op_add_node(None).unwrap();
        let first_input = program.op_add_node(None).unwrap();
        let second_input = program.op_add_node(None).unwrap();
        let multiple_nodes = program.op_add_node(None).unwrap();

        program.connect(input1, first_input, 0, None).unwrap();
        program.connect(input2, second_input, 1, None).unwrap();
        program.connect(input1, both_inputs, 0, None).unwrap();
        program.connect(input2, both_inputs, 1, None).unwrap();
        program.connect(input1, multiple_nodes, 0, None).unwrap();
        program.connect(input2, multiple_nodes, 0, None).unwrap();
        program.connect(input3, multiple_nodes, 1, None).unwrap();
        program.connect(input4, multiple_nodes, 1, None).unwrap();

        {
            let mat = MaterializedInputs::materialize(&program, no_inputs);
            assert_eq!(mat.inputs.len(), 0);
        }

        {
            let mut mat = MaterializedInputs::materialize(&program, first_input);
            assert_eq!(mat.inputs.len(), 1);
            assert_eq!(mat.inputs[0][0].source_node, input1);
            assert_eq!(mat.inputs[0][0].target_node, first_input);
            assert_eq!(mat.get_input(0)[0].source_node, input1);
            assert_eq!(mat.get_input(1).len(), 0);
            assert_eq!(mat.get_input_mut(1).len(), 0);
        }

        {
            let mat = MaterializedInputs::materialize(&program, second_input);
            assert_eq!(mat.inputs.len(), 2);
            assert_eq!(mat.get_input(0).len(), 0);
            assert_eq!(mat.get_input(1).len(), 1);
            assert_eq!(mat.get_input(1)[0].source_node, input2);
            assert_eq!(mat.get_input(1)[0].target_node, second_input);
        }

        {
            let mut mat = MaterializedInputs::materialize(&program, multiple_nodes);
            mat.get_input_mut(0).sort_unstable_by_key(|x| x.source_node);
            mat.get_input_mut(1).sort_unstable_by_key(|x| x.source_node);
            assert_eq!(mat.inputs.len(), 2);
            assert_eq!(mat.get_input(0)[0].source_node, input1);
            assert_eq!(mat.get_input(0)[1].source_node, input2);
            assert_eq!(mat.get_input(1)[0].source_node, input3);
            assert_eq!(mat.get_input(1)[1].source_node, input4);
        }
    }
}
