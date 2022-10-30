//! Implement the type inference pass.
//!
//! Type inference consists of unifying the incoming edges to a node's inputs to a type, and combining those types to
//! produce an output type for the node.  The output of this pass is the output types of all nodes, from which the types
//! of edges can trivially be inferred.
//!
//! This pass must run before insertion of the implicit add nodes and so must deal with the case of multiple incoming
//! edges per input.  This is because type inference is one of the final places in which good diagnostics must be
//! generated: if type inference succeeds, the program is valid and any bugs that make it invalid are on us, not the
//! user.
use std::collections::HashMap;

use crate::*;

/// Information on the types of nodes in a graph.
#[derive(Debug)]
pub struct TypeInfo {
    types: HashMap<OperationGraphNode, DataType>,
}

impl TypeInfo {
    pub fn get_type(&self, node: OperationGraphNode) -> Option<DataType> {
        self.types.get(&node).cloned()
    }
}

#[derive(Debug, thiserror::Error)]
#[error("Type inference failed")]
pub struct TypeInferenceError;

#[derive(Debug)]
enum TypeConstraint {
    /// The type of this node is exactly this value.
    ///
    /// `IsExactly` nodes may not look at their inputs at all, for example the final node doesn't care if the inputs are
    /// compatible with each other.
    IsExactly {
        data_type: DataType,
        cares_about_inputs: bool,
    },

    IsFromInput(usize),
    IsFromOutput(usize),
    IsFromProperty(usize),

    /// The node outputs this primitive, but the width must be inferred.
    IsPrimitive(PrimitiveType),
    /// The type of this node is inferred from the inputs, but must not be one of the listed primitives, or never.
    MustNotBePrimitive(&'static [PrimitiveType]),

    /// Infer the type from the node inputs; anything but Never is fine.
    FromNodeInputs,
}

#[derive(Debug)]
struct OpDescriptor {
    num_inputs: usize,

    constraint: TypeConstraint,
}

fn descriptor_for_op(op: &Op) -> OpDescriptor {
    match op {
        Op::Start => OpDescriptor {
            num_inputs: 0,
            constraint: TypeConstraint::IsExactly {
                data_type: DataType::Never,
                cares_about_inputs: true,
            },
        },
        Op::Final => OpDescriptor {
            num_inputs: 1,
            constraint: TypeConstraint::IsExactly {
                data_type: DataType::Never,
                cares_about_inputs: false,
            },
        },
        Op::Clock | Op::Sr => OpDescriptor {
            num_inputs: 0,
            constraint: TypeConstraint::IsExactly {
                data_type: DataType::Vector(VectorDescriptor::new_i64(1)),
                cares_about_inputs: true,
            },
        },
        Op::Constant(c) => OpDescriptor {
            num_inputs: 0,
            constraint: TypeConstraint::IsExactly {
                data_type: DataType::Vector(c.vector_descriptor()),
                cares_about_inputs: true,
            },
        },
        Op::Cast(prim) => OpDescriptor {
            num_inputs: 1,
            constraint: TypeConstraint::IsPrimitive(*prim),
        },
        Op::Negate => OpDescriptor {
            num_inputs: 1,
            constraint: TypeConstraint::MustNotBePrimitive(&[PrimitiveType::Bool]),
        },
        Op::BinOp(_) => OpDescriptor {
            num_inputs: 2,
            constraint: TypeConstraint::MustNotBePrimitive(&[PrimitiveType::Bool]),
        },
        Op::ReadInput(i) => OpDescriptor {
            num_inputs: 0,
            constraint: TypeConstraint::IsFromInput(*i),
        },
        Op::ReadProperty(p) => OpDescriptor {
            num_inputs: 0,
            constraint: TypeConstraint::IsFromProperty(*p),
        },
        Op::WriteOutput(o) => OpDescriptor {
            num_inputs: 1,
            constraint: TypeConstraint::IsFromOutput(*o),
        },
    }
}

pub fn type_inference(
    program: &Program,
    diagnostics: &mut DiagnosticCollection,
) -> Result<TypeInfo, TypeInferenceError> {
    let mut type_info = TypeInfo {
        types: Default::default(),
    };

    // We can type check nodes in the same order that we would if they were being run, so get a topological sort and use
    // that.
    let nodes = program.topological_sort().map_err(|d| {
        diagnostics.add_diagnostic(d);
        TypeInferenceError
    })?;

    // It is easier to get a failure count by counting successes, since we can use continue and not have to remember to
    // get counters in all the right places.
    let mut successes: usize = 0;

    // We can't actually produce useful information for nodes which have untyped inputs. In that case, let's report how
    // many nodes we couldn't check at all and that we gave up early.
    let mut uncheckable_count: usize = 0;

    'check_next: for n in nodes.iter().cloned() {
        let kind = program
            .graph
            .node_weight(n)
            .expect("We just did a topological sort");

        let descriptor = descriptor_for_op(&kind.op);

        // if the node is `IsExactly` and doesn't inspect inputs, just skip.
        if let TypeConstraint::IsExactly {
            cares_about_inputs: false,
            data_type,
        } = &descriptor.constraint
        {
            type_info.types.insert(n, *data_type);
            successes += 1;
            continue;
        }

        // We don't want the start node; that is never involved in type inference.
        let inputs =
            MaterializedInputs::materialize_with_filter(program, n, |x| x != program.start_node);

        if descriptor.num_inputs < inputs.inputs.len() {
            diagnostics.add_simple_diagnostic(
                program,
                format!(
                    "{}: found {} inputs, expected {}",
                    kind.op,
                    inputs.inputs.len(),
                    descriptor.num_inputs
                ),
                kind.source_loc.clone(),
            );

            continue;
        }

        if descriptor.num_inputs > inputs.inputs.len() {
            diagnostics.add_simple_diagnostic(
                program,
                format!(
                    "{}: needed {} inputs but only found {}",
                    kind.op,
                    descriptor.num_inputs,
                    inputs.inputs.len()
                ),
                kind.source_loc.clone(),
            );
            continue;
        }

        for i in 0..descriptor.num_inputs {
            if inputs.inputs[i].is_empty() {
                diagnostics.add_simple_diagnostic(
                    program,
                    format!("{}: missing input {}", kind.op, i),
                    kind.source_loc.clone(),
                );
                continue 'check_next;
            }
        }

        // For now we have only nodes which have inputs all of the same type, and which we can treat as collapsed into
        // one input. Infer the type, so we can uise it below.
        let all_inputs = inputs.inputs.iter().flat_map(|x| x.iter()).cloned();
        let mut unifier = None;
        for i in all_inputs {
            let ty = match type_info.get_type(i.source_node) {
                Some(t) => t,
                None => {
                    uncheckable_count += 1;
                    continue 'check_next;
                }
            };

            let vd = match ty {
                DataType::Vector(x) => x,
                DataType::Never => {
                    // Skip this. We are doing unification early, so this can come up.
                    continue;
                }
            };

            if unifier.is_none() {
                let disallowed =
                    if let TypeConstraint::MustNotBePrimitive(forbidden) = &descriptor.constraint {
                        Some(*forbidden)
                    } else {
                        None
                    };
                unifier = match crate::passes::unify_vectors::VectorUnifier::new(
                    program, n, vd, disallowed,
                ) {
                    Ok(u) => Some(u),
                    Err(d) => {
                        diagnostics.add_diagnostic(d);
                        continue 'check_next;
                    }
                }
            }
            let u = unifier
                .as_mut()
                .expect("We just initialized the unifier if needed");

            match u.present(program, n, vd) {
                Ok(()) => {}
                Err(d) => {
                    diagnostics.add_diagnostic(d);
                    continue 'check_next;
                }
            }
        }

        let unified_ty = match unifier {
            Some(u) => match u.resolve(program) {
                Ok(x) => Some(x),
                Err(d) => {
                    diagnostics.add_diagnostic(d);
                    continue;
                }
            },
            None => None,
        };

        let ty = match descriptor.constraint {
            TypeConstraint::IsExactly { data_type, .. } => data_type,
            TypeConstraint::IsFromInput(i) => match program.inputs.get(i) {
                Some(x) => DataType::Vector(*x),
                None => {
                    diagnostics.add_simple_diagnostic(
                        program,
                        format!(
                            "Attempt to read input {}, but only {} inputs available",
                            i,
                            program.inputs.len()
                        ),
                        kind.source_loc.clone(),
                    );
                    continue;
                }
            },
            TypeConstraint::IsFromProperty(i) => match program.properties.get(i) {
                Some(x) => DataType::Vector(VectorDescriptor::new(*x, 1)),
                None => {
                    diagnostics.add_simple_diagnostic(
                        program,
                        format!(
                            "Attempt to read property {}, but only {} properties available",
                            i,
                            program.properties.len()
                        ),
                        kind.source_loc.clone(),
                    );
                    continue;
                }
            },
            TypeConstraint::IsFromOutput(o) => {
                let expected = match program.outputs.get(o) {
                    Some(x) => DataType::Vector(*x),
                    None => {
                        diagnostics.add_simple_diagnostic(
                            program,
                            format!(
                                "Attempt to write output {}, but only {} outputs  available",
                                o,
                                program.outputs.len()
                            ),
                            kind.source_loc.clone(),
                        );
                        continue;
                    }
                };

                let has = unified_ty.expect("Output nodes have at least 1 input, so we will fail early if no unification is possible");
                if expected != DataType::Vector(has) {
                    diagnostics.add_simple_diagnostic(
                        program,
                        format!(
                            "Attempt to write output {}: expected {} but found {}",
                            o, expected, has
                        ),
                        kind.source_loc.clone(),
                    );
                    continue;
                }

                expected
            }
            TypeConstraint::IsPrimitive(prim) => {
                let got =
                    unified_ty.expect("Any nodes which must be a primitive have at least 1 input");

                DataType::new_vector(prim, got.width)
            }
            TypeConstraint::MustNotBePrimitive(prims) => {
                let got = unified_ty
                    .expect("Anything which must not be a specific primitive has 1 input");

                let ok = prims.iter().all(|prim| {
                    if *prim == got.primitive {
                        diagnostics.add_simple_diagnostic(
                            program,
                            format!("{} must not be a primitive of type {}", got, prim),
                            kind.source_loc.clone(),
                        );
                        false
                    } else {
                        true
                    }
                });

                if !ok {
                    // The diagnostic was already added.
                    continue;
                }

                DataType::Vector(got)
            }
            TypeConstraint::FromNodeInputs => {
                DataType::Vector(unified_ty.expect("This node type has at least 1 input"))
            }
        };

        type_info.types.insert(n, ty);
        successes += 1;
    }

    if successes != nodes.len() {
        if uncheckable_count > 0 {
            diagnostics.add_simple_diagnostic(
                program,
                format!(
                    "Type inference was unable to check {} nodes entirely; giving up",
                    uncheckable_count
                ),
                None,
            );
        }

        return Err(TypeInferenceError);
    }

    Ok(type_info)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[track_caller]
    fn type_program(prog: &mut Program) -> TypeInfo {
        let mut diags = DiagnosticCollection::new();
        crate::passes::insert_start_final_edges::insert_start_final_edges(prog, &mut diags)
            .unwrap();
        let res = type_inference(prog, &mut diags);
        assert!(res.is_ok(), "{}\n{}\n", prog.graphviz(), diags);
        res.unwrap()
    }

    #[test]
    fn test_basic() {
        let mut prog = Program::new();

        let i_i64_v1 = prog.add_input(PrimitiveType::I64, 1).unwrap();
        let i_f32_v2 = prog.add_input(PrimitiveType::F32, 2).unwrap();

        let p_i64_v1 = prog.add_property(PrimitiveType::I64).unwrap();
        let p_f32_v1 = prog.add_property(PrimitiveType::F32).unwrap();

        let o_i64_v2 = prog.add_output(PrimitiveType::I64, 2).unwrap();
        let o_f64_v2 = prog.add_output(PrimitiveType::F64, 2).unwrap();

        let read_input_i64_v1 = prog.op_read_input_node(i_i64_v1, None).unwrap();
        let read_input_f32_v2 = prog.op_read_input_node(i_f32_v2, None).unwrap();
        let read_prop_i64_v1 = prog.op_read_property_node(p_i64_v1, None).unwrap();
        let read_prop_f32_v1 = prog.op_read_property_node(p_f32_v1, None).unwrap();

        let write_output_i64_v2 = prog.op_write_output_node(o_i64_v2, None).unwrap();
        let write_output_f64_v2 = prog.op_write_output_node(o_f64_v2, None).unwrap();

        // We don't bother using anything beyond add because all BinOp are the same.

        // Negating should keep the type of the inputs.
        let negate_i64_v1 = prog.op_negate_node(None).unwrap();
        prog.connect(read_input_i64_v1, negate_i64_v1, 0, None)
            .unwrap();

        let cast_f64_v2 = prog.op_cast_node(PrimitiveType::F64, None).unwrap();
        prog.connect(read_input_f32_v2, cast_f64_v2, 0, None)
            .unwrap();

        let const_f64_v1 = prog
            .op_constant_node(Constant::F64(vec![0.0]), None)
            .unwrap();
        let broadcasted_add_f64_v2 = prog.op_add_node(None).unwrap();
        prog.connect(cast_f64_v2, broadcasted_add_f64_v2, 0, None)
            .unwrap();
        prog.connect(const_f64_v1, broadcasted_add_f64_v2, 0, None)
            .unwrap();
        let const_f64_v2 = prog
            .op_constant_node(Constant::F64(vec![0.0, 0.0]), None)
            .unwrap();
        prog.connect(const_f64_v2, broadcasted_add_f64_v2, 1, None)
            .unwrap();

        prog.connect(broadcasted_add_f64_v2, write_output_f64_v2, 0, None)
            .unwrap();

        let const_i64_v2 = prog
            .op_constant_node(Constant::I64(vec![0, 0]), None)
            .unwrap();
        prog.connect(const_i64_v2, write_output_i64_v2, 0, None)
            .unwrap();

        let typed = type_program(&mut prog);

        assert_eq!(
            typed.get_type(read_input_i64_v1),
            Some(DataType::new_v_i64(1))
        );
        assert_eq!(
            typed.get_type(read_input_f32_v2),
            Some(DataType::new_v_f32(2))
        );
        assert_eq!(
            typed.get_type(read_prop_i64_v1),
            Some(DataType::new_v_i64(1))
        );
        assert_eq!(
            typed.get_type(read_prop_f32_v1),
            Some(DataType::new_v_f32(1))
        );
        assert_eq!(typed.get_type(const_f64_v1), Some(DataType::new_v_f64(1)));
        assert_eq!(typed.get_type(cast_f64_v2), Some(DataType::new_v_f64(2)));
        assert_eq!(
            typed.get_type(broadcasted_add_f64_v2),
            Some(DataType::new_v_f64(2))
        );
        assert_eq!(
            typed.get_type(write_output_i64_v2),
            Some(DataType::new_v_i64(2))
        );
        assert_eq!(
            typed.get_type(write_output_f64_v2),
            Some(DataType::new_v_f64(2))
        );
    }

    #[track_caller]
    fn assert_fails_typing(prog: &mut Program) {
        let mut diags = DiagnosticCollection::new();
        crate::passes::insert_start_final_edges::insert_start_final_edges(prog, &mut diags)
            .unwrap();
        let res = type_inference(prog, &mut diags);
        assert!(res.is_err(), "{}\n", diags);
    }

    #[test]
    fn test_too_few_inputs() {
        let mut prog = Program::new();
        let c1 = prog.op_constant_node(Constant::I64(vec![0]), None).unwrap();
        let adder = prog.op_add_node(None).unwrap();
        prog.connect(c1, adder, 0, None).unwrap();
        assert_fails_typing(&mut prog);
    }

    #[test]
    fn test_missing_inputs() {
        let mut prog = Program::new();
        let c1 = prog.op_constant_node(Constant::I64(vec![0]), None).unwrap();
        let adder = prog.op_add_node(None).unwrap();
        prog.connect(c1, adder, 1, None).unwrap();
        assert_fails_typing(&mut prog);
    }

    #[test]
    fn test_too_many_inputs() {
        let mut prog = Program::new();
        let c1 = prog.op_constant_node(Constant::I64(vec![0]), None).unwrap();
        let adder = prog.op_add_node(None).unwrap();
        for i in 0..5 {
            prog.connect(c1, adder, i, None).unwrap();
        }
        assert_fails_typing(&mut prog);
    }

    #[test]
    fn test_primitive_mismatch_writing_output() {
        let mut prog = Program::new();
        let o = prog.add_output(PrimitiveType::F32, 2).unwrap();
        let writer = prog.op_write_output_node(o, None).unwrap();
        let constant = prog
            .op_constant_node(Constant::I64(vec![0, 0]), None)
            .unwrap();
        prog.connect(constant, writer, 0, None).unwrap();
        assert_fails_typing(&mut prog);
    }

    #[test]
    fn test_width_mismatch_writing_output() {
        let mut prog = Program::new();
        let o = prog.add_output(PrimitiveType::F32, 2).unwrap();
        let writer = prog.op_write_output_node(o, None).unwrap();
        let constant = prog
            .op_constant_node(Constant::I64(vec![0, 0, 0]), None)
            .unwrap();
        prog.connect(constant, writer, 0, None).unwrap();
        assert_fails_typing(&mut prog);
    }

    #[test]
    fn test_no_inputs_to_clock() {
        let mut prog = Program::new();
        let clock = prog.op_clock_node(None).unwrap();
        let constant = prog
            .op_constant_node(Constant::I64(vec![1, 1]), None)
            .unwrap();
        prog.connect(constant, clock, 0, None).unwrap();
        assert_fails_typing(&mut prog);
    }

    #[test]
    fn test_no_inputs_to_sr() {
        let mut prog = Program::new();
        let sr = prog.op_sr_node(None).unwrap();
        let constant = prog
            .op_constant_node(Constant::I64(vec![1, 1]), None)
            .unwrap();
        prog.connect(constant, sr, 0, None).unwrap();
        assert_fails_typing(&mut prog);
    }
}
