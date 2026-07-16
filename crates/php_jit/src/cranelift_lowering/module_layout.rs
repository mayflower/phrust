//! Deterministic whole-unit native module layout.
//!
//! A PHP function body belongs to exactly one baseline module generation for
//! its source unit. Direct calls inside that unit use module-local symbols;
//! published aliases select those same bodies without recompiling a bounded
//! transitive graph for every possible root.

use php_ir::{FunctionId, IrUnit};

/// Returns every function in stable source order, with `root` first.
pub(super) fn whole_unit_function_order(unit: &IrUnit, root: FunctionId) -> Vec<FunctionId> {
    let mut functions = Vec::with_capacity(unit.functions.len());
    if unit.functions.get(root.index()).is_some() {
        functions.push(root);
    }
    functions.extend(unit.functions.iter().enumerate().filter_map(|(index, _)| {
        let function = FunctionId::new(u32::try_from(index).ok()?);
        (function != root).then_some(function)
    }));
    functions
}

#[cfg(test)]
mod tests {
    use super::*;
    use php_ir::{FunctionFlags, IrBuilder, IrSpan, UnitId};

    #[test]
    fn whole_unit_order_is_root_first_then_source_order() {
        let mut builder = IrBuilder::new(UnitId::new(1));
        let file = builder.add_file("layout.php");
        let span = IrSpan::new(file, 0, 1);
        for name in ["first", "second", "third"] {
            let function = builder.start_function(name, FunctionFlags::default(), span);
            let block = builder.append_block(function);
            builder.terminate_return(function, block, None, span);
        }
        let unit = builder.finish();

        assert_eq!(
            whole_unit_function_order(&unit, FunctionId::new(1)),
            vec![FunctionId::new(1), FunctionId::new(0), FunctionId::new(2)]
        );
    }
}
