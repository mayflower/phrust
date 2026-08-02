fn reject_unpublished_optimizer_boundaries(
    region: &RegionGraph,
) -> Result<(), CraneliftLoweringError> {
    for instruction in region.blocks.iter().flat_map(|block| &block.instructions) {
        match &instruction.kind {
            RegionInstructionKind::NativeDynamicCode(_) => {
                return Err(CraneliftLoweringError::new(
                    "JIT_CRANELIFT_REJECT_DYNAMIC_CODE_PUBLICATION",
                    format!(
                        "dynamic-code continuation {} must enter the Generic tier before optimizing execution",
                        instruction.continuation_id,
                    ),
                ));
            }
            RegionInstructionKind::NativeSuspend(RegionNativeSuspend::FiberSuspend { .. }) => {
                return Err(CraneliftLoweringError::new(
                    "JIT_CRANELIFT_REJECT_FIBER_OWNERSHIP_PUBLICATION",
                    format!(
                        "fiber suspension at continuation {} has no total optimizing ownership plan",
                        instruction.continuation_id,
                    ),
                ));
            }
            RegionInstructionKind::NativeCall(call) => {
                if matches!(
                    call.target,
                    RegionCallTarget::Callable { .. }
                        | RegionCallTarget::Closure { function: None, .. }
                        | RegionCallTarget::Pipe { .. }
                ) {
                    return Err(CraneliftLoweringError::new(
                        "JIT_CRANELIFT_REJECT_DYNAMIC_CALLABLE_PUBLICATION",
                        format!(
                            "dynamic callable at continuation {} has no publication-fixed target/signature",
                            instruction.continuation_id,
                        ),
                    ));
                }
                if stable_builtin_format(&call.target).is_some()
                    || stable_builtin_compression_codec(&call.target).is_some()
                    || stable_builtin_callable_query(&call.target).is_some()
                    || stable_builtin_settype(&call.target)
                {
                    return Err(CraneliftLoweringError::new(
                        "JIT_CRANELIFT_REJECT_EXACT_BUILTIN_RESOURCE_PUBLICATION",
                        format!(
                            "resource-dependent exact builtin at continuation {} has no bounded total output plan",
                            instruction.continuation_id,
                        ),
                    ));
                }
                if let RegionCallTarget::Semantic { operation } = &call.target {
                    if matches!(
                        operation,
                        RegionSemanticOp::StaticPropertyFetch { .. }
                            | RegionSemanticOp::StaticPropertyAssign { .. }
                            | RegionSemanticOp::StaticPropertyIsset { .. }
                            | RegionSemanticOp::StaticPropertyEmpty { .. }
                            | RegionSemanticOp::StaticPropertyDimIsset { .. }
                            | RegionSemanticOp::StaticPropertyDimEmpty { .. }
                            | RegionSemanticOp::StaticPropertyDimUnset { .. }
                            | RegionSemanticOp::StaticPropertyReference { .. }
                    ) {
                        return Err(CraneliftLoweringError::new(
                            "JIT_CRANELIFT_REJECT_STATIC_PROPERTY_PUBLICATION",
                            format!(
                                "static-property continuation {} has no total visibility/slot publication plan",
                                instruction.continuation_id,
                            ),
                        ));
                    }
                    if matches!(
                        operation,
                        RegionSemanticOp::ClassConstantFetch { class_name, .. }
                            if matches!(
                                class_name.trim_start_matches('\\').to_ascii_lowercase().as_str(),
                                "self" | "parent" | "static"
                            )
                    ) {
                        return Err(CraneliftLoweringError::new(
                            "JIT_CRANELIFT_REJECT_RELATIVE_CLASS_CONSTANT_PUBLICATION",
                            format!(
                                "relative class-constant continuation {} has no total calling-class publication plan",
                                instruction.continuation_id,
                            ),
                        ));
                    }
                }
            }
            _ => {}
        }
    }
    Ok(())
}
