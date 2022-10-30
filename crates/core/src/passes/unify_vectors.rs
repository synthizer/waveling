use crate::{
    Diagnostic, DiagnosticBuilder, OperationGraphNode, PrimitiveType, Program,
    SingleErrorResult as Result, VectorDescriptor,
};

/// A type for unifying vectors.
///
/// This inverted interface eases testing.  Given a set of input nodes, compute a final type, or error out.
///
/// For now, we ensure correctness by forcing constructors to pass an initial node/type pairing in.
pub struct VectorUnifier<'a> {
    descriptor: VectorDescriptor,

    /// Helps build error reports.
    last_node: OperationGraphNode,

    /// Denied primitives, if any.
    denied_primitives: Option<&'a [PrimitiveType]>,
}

fn build_primitive_type_mismatch_err(
    program: &Program,
    node1: OperationGraphNode,
    prim1: PrimitiveType,
    node2: OperationGraphNode,
    prim2: PrimitiveType,
) -> Diagnostic {
    let mut builder = DiagnosticBuilder::new(
        format!(
            "Primitive type mismatch. Expected {} but found {}",
            prim1, prim2
        ),
        None,
    );
    builder.node_ref(format!("This node is {}", prim1), node1);
    builder.node_ref(
        format!(
            "But this node is {}, which is of a different primitive type",
            prim2
        ),
        node2,
    );
    builder.build(program)
}

fn build_broadcasting_error(
    program: &Program,
    node1: OperationGraphNode,
    desc1: &VectorDescriptor,
    node2: OperationGraphNode,
    desc2: &VectorDescriptor,
) -> Diagnostic {
    let mut builder = DiagnosticBuilder::new(
        format!("Unable to broadcast from {} to {}", desc1, desc2),
        None,
    );
    builder.node_ref(format!("This node is a {}", desc1), node1);
    builder.node_ref(format!("But this node is a {}", desc2), node2);
    builder.build(program)
}

fn build_zero_width_error(
    program: &Program,
    node: OperationGraphNode,
    desc: &VectorDescriptor,
) -> Diagnostic {
    let mut builder = DiagnosticBuilder::new(
        "Nodes which carry data must not use vectors of zero width",
        None,
    );
    builder.node_ref(format!("This node is a {}", desc), node);
    builder.build(program)
}

fn validate_primitive(
    prog: &Program,
    node: OperationGraphNode,
    prim: PrimitiveType,
    denied_primitives: Option<&[PrimitiveType]>,
) -> Result<()> {
    match denied_primitives {
        None => Ok(()),
        Some(s) if !s.contains(&prim) => Ok(()),
        _ => {
            let mut builder = DiagnosticBuilder::new(
                format!("Unsupported primitive type {} for node", prim),
                None,
            );
            builder.node_ref("This is the node of the disallowed type", node);
            Err(builder.build(prog))
        }
    }
}

impl<'a> VectorUnifier<'a> {
    pub fn new(
        program: &Program,
        node: OperationGraphNode,
        descriptor: VectorDescriptor,
        denied_primitives: Option<&'a [PrimitiveType]>,
    ) -> Result<Self> {
        validate_primitive(program, node, descriptor.primitive, denied_primitives)?;

        assert!(program.graph.node_weight(node).is_some());

        if descriptor.width == 0 {
            return Err(build_zero_width_error(program, node, &descriptor));
        }
        let ret = Self {
            last_node: node,
            descriptor,
            denied_primitives,
        };

        Ok(ret)
    }

    pub fn present(
        &mut self,
        program: &Program,
        node: OperationGraphNode,
        descriptor: VectorDescriptor,
    ) -> Result<()> {
        validate_primitive(program, node, descriptor.primitive, self.denied_primitives)?;

        if self.descriptor.primitive != descriptor.primitive {
            return Err(build_primitive_type_mismatch_err(
                program,
                self.last_node,
                self.descriptor.primitive,
                node,
                descriptor.primitive,
            ));
        }

        if descriptor.width == 0 {
            return Err(build_zero_width_error(program, node, &descriptor));
        }

        let can_broadcast = self.descriptor.width == 1 || descriptor.width == 1;
        if !can_broadcast && self.descriptor.width != descriptor.width {
            return Err(build_broadcasting_error(
                program,
                self.last_node,
                &self.descriptor,
                node,
                &descriptor,
            ));
        }

        if self.descriptor.width < descriptor.width {
            self.descriptor = descriptor;
        }

        self.last_node = node;
        Ok(())
    }

    pub fn resolve(self, _program: &Program) -> Result<VectorDescriptor> {
        Ok(self.descriptor)
    }
}

#[cfg(test)]
mod tests {
    pub use super::*;

    /// Run a unification against a set of types, returning the final result.
    fn run_unification(
        tys: &[VectorDescriptor],
        denied_tys: Option<&'static [PrimitiveType]>,
    ) -> Result<VectorDescriptor> {
        let mut prog = Program::new();
        let fake_index = prog.op_add_node(None).unwrap();
        let mut unifier = VectorUnifier::new(&prog, fake_index, tys[0], denied_tys)?;
        for t in tys.iter().skip(1) {
            unifier.present(&prog, fake_index, *t)?;
        }

        unifier.resolve(&prog)
    }

    #[test]
    fn test_no_primitive_denial() {
        use VectorDescriptor as VD;

        // OK: only one descriptor.
        assert_eq!(
            run_unification(&[VD::new_bool(1)], None).unwrap(),
            VD::new_bool(1)
        );

        // Ok: all are the same width and type.
        assert_eq!(
            run_unification(&[VD::new_i64(3), VD::new_i64(3), VD::new_i64(3)], None).unwrap(),
            VD::new_i64(3)
        );

        // Constants can broadcast out.
        assert_eq!(
            run_unification(&[VD::new_f32(1), VD::new_f32(4)], None).unwrap(),
            VD::new_f32(4)
        );
        assert_eq!(
            run_unification(&[VD::new_f32(4), VD::new_f32(1)], None).unwrap(),
            VD::new_f32(4)
        );
        assert_eq!(
            run_unification(
                &[
                    VD::new_f32(1),
                    VD::new_f32(4),
                    VD::new_f32(1),
                    VD::new_f32(4)
                ],
                None
            )
            .unwrap(),
            VD::new_f32(4)
        );

        // Zero-width vectors always fail out.
        assert!(run_unification(&[VD::new_f32(0), VD::new_f32(4)], None).is_err());
        assert!(run_unification(&[VD::new_f32(1), VD::new_f32(0)], None).is_err());

        // Changing the primitive must also fail.
        assert!(run_unification(&[VD::new_f32(1), VD::new_f64(1)], None).is_err());
    }

    #[test]
    fn test_denying_primitives() {
        use PrimitiveType::*;
        use VectorDescriptor as VD;

        // Denying a primitive that isn't in the list of types is fine.
        assert!(run_unification(&[VD::new_f32(5), VD::new_f32(5)], Some(&[I64])).is_ok());

        // We want to check denial in both positions, because we actually add the check in two places.
        //
        // We can furthermore use the fact that we have a substring in the error we generate to verify we got the right
        // error.
        for tys in [
            [VD::new_f32(5), VD::new_i64(5)],
            [VD::new_i64(5), VD::new_f32(5)],
        ] {
            assert!(run_unification(&tys[..], Some(&[I64]))
                .err()
                .unwrap()
                .to_string()
                .contains("Unsupported primitive"));
        }
    }
}
