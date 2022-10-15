use petgraph::prelude::*;

use crate::*;

#[derive(thiserror::Error, Debug)]
#[error(
    "insert_start_final_edges pass failed. Diagnostics have been pushed to the DiagnosticBuilder"
)]
pub struct InsertStartFinalEdgesError;

/// What kind of implicit edges does this operation have?
///
/// This is used to feed setup of the edges from the start and final nodes rather than having logic scattered all over;
/// declarative is easier to reason about.
#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash, derive_more::IsVariant)]
enum ImplicitEdgeKind {
    /// All edges for this node must be declared by the user.
    None,

    /// This node implicitly conects to the start node.
    Start,

    /// This node implicitly connecs to the final node.
    Final,
}

fn implicit_edge_kind(o: &Op) -> ImplicitEdgeKind {
    use self::ImplicitEdgeKind::*;
    match o {
        Op::Start | Op::Final => None,
        Op::ReadInput(_) | Op::Clock | Op::Sr | Op::ReadProperty(_) | Op::Constant(_) => Start,
        Op::Negate | Op::BinOp(_) | Op::Cast(_) | Op::ReadState { .. } => None,
        Op::WriteOutput(_) | Op::WriteState { .. } => Final,
    }
}

fn node_has_edge_from_kind<'a>(
    program: &Program,
    edges: impl Iterator<Item = OperationGraphEdgeRef<'a>>,
    pred: impl FnMut(&Op) -> bool,
) -> bool {
    edges
        .map(|e| &program.graph.node_weight(e.source()).unwrap().op)
        .any(pred)
}
/// Run the pass which inserts edges from the start node to all initial nodes in the graph, and inserts edges from all
/// the final nodes to the implicit final node.
///
/// If This pass fails, it has pushed the appropriate diagnostics to the builder already.
pub fn insert_start_final_edges(
    program: &mut Program,
    diagnostics: &mut DiagnosticCollection,
) -> Result<(), InsertStartFinalEdgesError> {
    // First grab a topological sort of the graph.
    let nodes = program.topological_sort().map_err(|e| {
        diagnostics.add_diagnostic(e);
        InsertStartFinalEdgesError
    })?;

    // Now perform some validation, before we go throwing edges all over the place.
    //
    // It is an error if the graph has an edge to the start or final node from any node which doesn't have implicit
    // edges, or if it's in the wrong direction.
    //
    // We want to do as much validation as possible so that the diagnostics are good.
    let mut validation_succeeded = true;
    for node in nodes.iter() {
        let (needs_start, needs_final) =
            match implicit_edge_kind(&program.graph.node_weight(*node).unwrap().op) {
                ImplicitEdgeKind::None => (false, false),
                ImplicitEdgeKind::Start => (true, false),
                ImplicitEdgeKind::Final => (false, true),
            };

        let has_start = node_has_edge_from_kind(
            program,
            program.graph.edges_directed(*node, Direction::Incoming),
            Op::is_start,
        );

        let has_final = node_has_edge_from_kind(
            program,
            program.graph.edges_directed(*node, Direction::Incoming),
            Op::is_final,
        );

        // Now we must do some error checking.

        // This logic is predicated on the fact that we currently only have a set of operations which doesn't allow for
        // a program of one node, or where an unpaired operation can be "off to the side".  Put another way, programs
        // consist of reads and writes which are both separate nodes, and short of dead code every read pairs with a
        // write later in the control flow graph.
        //
        // If this ever changes, e.g. we decide to add some sort of logger or metrics or idk what, this logic will need
        // to be amended.

        assert!(!needs_start || !needs_final);
        let err: Option<&str> = if needs_start && has_final {
            Some("Nodes which connect to the start node must not be connected to the final node")
        } else if has_start && needs_final {
            Some("Nodes which end the program must not connect to the start node")
        } else if has_start && has_final {
            Some("Nodes cannot be connected both to the start and final node at the same time")
        } else {
            None
        };

        if let Some(err) = err {
            let mut db = DiagnosticBuilder::new(err, None);
            db.node_ref("The problematic node", *node);
            diagnostics.add_diagnostic(db.build(program));
            validation_succeeded = false;
        }
    }

    if !validation_succeeded {
        return Err(InsertStartFinalEdgesError);
    }

    // Now we just do the simple loop.
    for node in nodes.iter() {
        let implicit_kind = implicit_edge_kind(&program.graph.node_weight(*node).unwrap().op);

        match implicit_kind {
            ImplicitEdgeKind::None => {}
            ImplicitEdgeKind::Start => {
                program.graph.update_edge(
                    program.start_node,
                    *node,
                    Edge {
                        input: 0,
                        source_loc: None,
                    },
                );
            }
            ImplicitEdgeKind::Final => {
                program.graph.update_edge(
                    *node,
                    program.final_node,
                    Edge {
                        input: 0,
                        source_loc: None,
                    },
                );
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_implicit_edges() {
        use itertools::Itertools;

        let mut program = Program::new();
        let input_index = program.add_input(PrimitiveType::F32, 3).unwrap();
        let output_index = program.add_output(PrimitiveType::F32, 3).unwrap();
        let prop_index = program.add_property(PrimitiveType::F32).unwrap();
        let state_index = program.add_state(PrimitiveType::F32, 3, 10).unwrap();

        // These nodes should have an edge from the start node.  Put them in an array, then reduce that array into an
        // add node, then connect that add node to the ones that should have an edge to the final node.
        let starts = vec![
            program.op_read_input_node(input_index, None).unwrap(),
            program
                .op_constant_node(Constant::F32(vec![0.0, 0.0, 0.0]), None)
                .unwrap(),
            program.op_clock_node(None).unwrap(),
            program.op_sr_node(None).unwrap(),
            program.op_read_property_node(prop_index, None).unwrap(),
        ];

        let mut adds = vec![];
        let final_add = starts
            .iter()
            .cloned()
            .tree_fold1(|a, b| {
                let add = program.op_add_node(None).unwrap();
                program.connect(a, add, 0, None).unwrap();
                program.connect(b, add, 1, None).unwrap();
                adds.push(add);
                add
            })
            .unwrap();

        let ends = vec![
            program
                .op_write_state_direct_node(state_index, None)
                .unwrap(),
            program.op_write_output_node(output_index, None).unwrap(),
        ];

        for n in ends.iter().cloned() {
            program.connect(final_add, n, 0, None).unwrap();
        }

        insert_start_final_edges(&mut program, &mut DiagnosticCollection::new()).unwrap();

        let gv = program.graphviz();

        // All of the start nodes should have an incoming edge from the initial node, and no edges to the final node.
        for n in starts.iter().cloned() {
            assert!(program.graph.contains_edge(program.start_node, n), "{}", gv);
            assert!(
                !program.graph.contains_edge(n, program.final_node),
                "{}",
                gv
            );
        }

        // The adds should never go to either.
        for n in adds.iter().cloned() {
            assert!(
                !program.graph.contains_edge(program.start_node, n),
                "{}",
                gv
            );
            assert!(
                !program.graph.contains_edge(n, program.final_node),
                "{}",
                gv
            );
        }

        // And the final nodes never to the starts.
        for n in starts.iter().cloned() {
            assert!(
                !program.graph.contains_edge(program.final_node, n),
                "{}",
                gv
            );
        }
    }
}
