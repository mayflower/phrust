use php_ir::{
    FunctionFlags, InstructionKind, IrBuilder, IrConstant, IrSpan, LoweringOptions, Operand,
    UnitId, lower_frontend_result, verify_unit,
};
use php_semantics::analyze_source;

#[test]
fn manual_basic_ir_snapshot_is_stable() {
    let unit = manual_basic_unit();
    verify_unit(&unit).expect("manual snapshot unit should verify");
    let actual = unit.to_snapshot_text();
    let expected = include_str!("../../../fixtures/bytecode/valid/manual-basic.ir.snap");
    assert_eq!(actual, expected);
}

#[test]
fn manual_basic_ir_json_is_available() {
    let unit = manual_basic_unit();
    let json = unit.to_json_pretty().expect("manual IR should serialize");
    assert!(json.contains("\"version\": 1"));
    assert!(json.contains("\"functions\""));
    assert!(json.contains("\"opcode\": \"binary\""));
}

#[test]
fn lowered_single_literal_snapshot_is_stable() {
    let actual = lowered_snapshot(
        "<?php echo 1;",
        "fixtures/bytecode/literals/valid/echo-int.php",
    );
    let expected = include_str!("../../../fixtures/bytecode/valid/literals-single.ir.snap");
    assert_eq!(actual, expected);
}

#[test]
fn lowered_multiple_literals_snapshot_is_stable() {
    let actual = lowered_snapshot(
        "<?php echo 1, \"x\";",
        "fixtures/bytecode/literals/valid/echo-multiple.php",
    );
    let expected = include_str!("../../../fixtures/bytecode/valid/literals-multiple.ir.snap");
    assert_eq!(actual, expected);
}

#[test]
fn lowered_source_map_snapshot_is_stable() {
    let actual = lowered_snapshot(
        "<?php echo null, true;",
        "fixtures/bytecode/literals/valid/echo-source-map.php",
    );
    let expected = include_str!("../../../fixtures/bytecode/valid/source-map.ir.snap");
    assert_eq!(actual, expected);
}

#[test]
fn lowered_foreach_snapshot_is_stable() {
    let source = include_str!("../../../fixtures/bytecode/lower/valid/foreach.php");
    let actual = lowered_snapshot(source, "fixtures/bytecode/lower/valid/foreach.php");
    let expected = include_str!("../../../fixtures/bytecode/valid/foreach.ir.snap");
    assert_eq!(actual, expected);
}

#[test]
fn lowered_include_snapshot_is_stable() {
    let source = include_str!("../../../fixtures/bytecode/lower/valid/include.php");
    let actual = lowered_snapshot(source, "fixtures/bytecode/lower/valid/include.php");
    let expected = include_str!("../../../fixtures/bytecode/valid/include.ir.snap");
    assert_eq!(actual, expected);
}

fn manual_basic_unit() -> php_ir::IrUnit {
    let mut builder = IrBuilder::new(UnitId::new(0));
    let file = builder.add_file("fixtures/runtime/valid/scalars/echo.php");
    let function = builder.start_function(
        "main",
        FunctionFlags {
            is_top_level: true,
            ..FunctionFlags::default()
        },
        IrSpan::new(file, 0, 5),
    );
    let block = builder.append_block(function);
    let one = builder.add_constant(IrConstant::Int(1));
    let two = builder.add_constant(IrConstant::Int(2));
    let r0 = builder.alloc_register(function);
    let r1 = builder.alloc_register(function);
    let r2 = builder.alloc_register(function);
    builder.emit_load_const(function, block, r0, one, IrSpan::new(file, 6, 7));
    builder.emit_load_const(function, block, r1, two, IrSpan::new(file, 10, 11));
    builder.emit(
        function,
        block,
        InstructionKind::Binary {
            dst: r2,
            op: php_ir::BinaryOp::Add,
            lhs: Operand::Register(r0),
            rhs: Operand::Register(r1),
        },
        IrSpan::new(file, 6, 11),
    );
    builder.terminate_return(
        function,
        block,
        Some(Operand::Register(r2)),
        IrSpan::new(file, 6, 11),
    );
    builder.set_entry(function);
    builder.finish()
}

fn lowered_snapshot(source: &str, source_path: &str) -> String {
    let frontend = analyze_source(source);
    let result = lower_frontend_result(
        &frontend,
        LoweringOptions {
            source_path: source_path.to_string(),
            ..LoweringOptions::default()
        },
    );
    result
        .verification
        .expect("lowered snapshot unit should verify");
    assert!(result.diagnostics.is_empty(), "{:#?}", result.diagnostics);
    result.unit.to_snapshot_text()
}
