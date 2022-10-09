use crate::{
    Diagnostic, DiagnosticBuilder, OperationGraphNode, PrimitiveType, Program,
    SingleErrorResult as Result, VectorDescriptor,
};

/// A type for unifying vectors.
///
/// This inverted interface eases testing.  Given a set of input nodes, compute a final type, or error out.
///
/// For now, we ensure correctness by forcing constructors to pass an initial node/type pairing in.
pub struct VectorUnifier {
    descriptor: VectorDescriptor,

    /// Helps build error reports.
    last_node: OperationGraphNode,
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
    builder.node_ref(format!("This node is a {}", prim1), node1);
    builder.node_ref(
        format!(
            "But this node is a {}, which is of a different primitive type",
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

impl VectorUnifier {
    pub fn new(
        program: &Program,
        node: OperationGraphNode,
        descriptor: VectorDescriptor,
    ) -> Result<Self> {
        assert!(program.graph.node_weight(node).is_some());

        if descriptor.width == 0 {
            return Err(build_zero_width_error(program, node, &descriptor));
        }
        Ok(Self {
            last_node: node,
            descriptor,
        })
    }

    pub fn present(
        &mut self,
        program: &Program,
        node: OperationGraphNode,
        descriptor: VectorDescriptor,
    ) -> Result<()> {
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
    fn run_unification(tys: &[VectorDescriptor]) -> Result<VectorDescriptor> {
        let mut prog = Program::new();
        let fake_index = prog.op_add_node(None).unwrap();
        let mut unifier = VectorUnifier::new(&prog, fake_index, tys[0])?;
        for t in tys.iter().skip(1) {
            unifier.present(&prog, fake_index, *t)?;
        }

        unifier.resolve(&prog)
    }

    #[test]
    fn test_unify_vectors() {
        use VectorDescriptor as VD;

        // OK: only one descriptor.
        assert_eq!(
            run_unification(&[VD::new_bool(1)]).unwrap(),
            VD::new_bool(1)
        );

        // Ok: all are the same width and type.
        assert_eq!(
            run_unification(&[VD::new_i64(3), VD::new_i64(3), VD::new_i64(3)]).unwrap(),
            VD::new_i64(3)
        );

        // Constants can broadcast out.
        assert_eq!(
            run_unification(&[VD::new_f32(1), VD::new_f32(4)]).unwrap(),
            VD::new_f32(4)
        );
        assert_eq!(
            run_unification(&[VD::new_f32(4), VD::new_f32(1)]).unwrap(),
            VD::new_f32(4)
        );
        assert_eq!(
            run_unification(&[
                VD::new_f32(1),
                VD::new_f32(4),
                VD::new_f32(1),
                VD::new_f32(4)
            ])
            .unwrap(),
            VD::new_f32(4)
        );

        // Zero-width vectors always fail out.
        assert!(run_unification(&[VD::new_f32(0), VD::new_f32(4)]).is_err());
        assert!(run_unification(&[VD::new_f32(1), VD::new_f32(0)]).is_err());

        // Changing the primitive must also fail.
        assert!(run_unification(&[VD::new_f32(1), VD::new_f64(1)]).is_err());
    }
}
