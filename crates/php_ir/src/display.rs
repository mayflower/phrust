//! Stable display helpers for IR units.

use crate::constants::IrConstant;
use crate::function::IrReturnType;
use crate::instruction::{BinaryOp, CastKind, CompareOp, InstructionKind, TerminatorKind, UnaryOp};
use crate::module::IrUnit;
use crate::operand::Operand;
use crate::source_map::{IrSourceMapTarget, IrSpan};
use std::fmt::{self, Write};

impl IrUnit {
    /// Returns a deterministic text representation for snapshots.
    #[must_use]
    pub fn to_snapshot_text(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(
            out,
            "ir version={} unit={} entry=function:{}",
            self.version,
            self.id.raw(),
            self.entry.raw()
        );
        let _ = writeln!(out, "files:");
        for file in &self.files {
            let _ = writeln!(out, "  file:{} {:?}", file.id.raw(), file.path);
        }
        let _ = writeln!(out, "constants:");
        for (index, constant) in self.constants.iter().enumerate() {
            let _ = writeln!(out, "  const:{index} {}", format_constant(constant));
        }
        let _ = writeln!(out, "constant_table:");
        for entry in &self.constant_table {
            let _ = writeln!(
                out,
                "  constant_name {:?} => const:{} span={}",
                entry.name,
                entry.value.raw(),
                format_span(entry.span)
            );
        }
        let _ = writeln!(out, "classes:");
        for class in &self.classes {
            let _ = writeln!(
                out,
                "  class:{} {:?} methods={} properties={} constructor={} flags=abstract:{} final:{} readonly:{} span={}",
                class.id.raw(),
                class.name,
                class.methods.len(),
                class.properties.len(),
                class
                    .constructor
                    .map(|function| format!("function:{}", function.raw()))
                    .unwrap_or_else(|| "none".to_owned()),
                class.flags.is_abstract,
                class.flags.is_final,
                class.flags.is_readonly,
                format_span(class.span)
            );
            for method in &class.methods {
                let _ = writeln!(
                    out,
                    "    method {:?} => function:{} static:{} private:{} protected:{} abstract:{}",
                    method.name,
                    method.function.raw(),
                    method.flags.is_static,
                    method.flags.is_private,
                    method.flags.is_protected,
                    method.flags.is_abstract
                );
            }
            for property in &class.properties {
                let _ = writeln!(
                    out,
                    "    property ${} default={} type={} static:{} private:{} protected:{} readonly:{} typed:{}",
                    property.name,
                    property
                        .default
                        .map(|constant| format!("const:{}", constant.raw()))
                        .unwrap_or_else(|| "none".to_owned()),
                    property
                        .type_
                        .as_ref()
                        .map(format_return_type)
                        .unwrap_or_else(|| "none".to_owned()),
                    property.flags.is_static,
                    property.flags.is_private,
                    property.flags.is_protected,
                    property.flags.is_readonly,
                    property.flags.is_typed
                );
            }
        }
        let _ = writeln!(out, "function_table:");
        for entry in &self.function_table {
            let _ = writeln!(
                out,
                "  function_name {:?} => function:{}",
                entry.name,
                entry.function.raw()
            );
        }
        let _ = writeln!(out, "functions:");
        for function in &self.functions {
            let _ = writeln!(
                out,
                "  function {:?} params={} locals={} regs={} flags={} span={}",
                function.name,
                function.params.len(),
                function.local_count,
                function.register_count,
                format_flags(
                    function.flags.is_top_level,
                    function.flags.is_closure,
                    function.flags.is_method
                ),
                format_span(function.span)
            );
            if let Some(return_type) = &function.return_type {
                let _ = writeln!(out, "    return_type {}", format_return_type(return_type));
            }
            for capture in &function.captures {
                let _ = writeln!(
                    out,
                    "    capture {:?} local:{} by_ref={}",
                    capture.name,
                    capture.local.raw(),
                    capture.by_ref
                );
            }
            for param in &function.params {
                let _ = writeln!(
                    out,
                    "    param {:?} local:{} required={} variadic={} by_ref={} type={} default={}",
                    param.name,
                    param.local.raw(),
                    param.required,
                    param.variadic,
                    param.by_ref,
                    param
                        .type_
                        .as_ref()
                        .map(format_return_type)
                        .unwrap_or_else(|| "none".to_string()),
                    param
                        .default
                        .as_ref()
                        .map(format_constant)
                        .unwrap_or_else(|| "none".to_string())
                );
            }
            for (index, local) in function.locals.iter().enumerate() {
                let _ = writeln!(out, "    local:{} ${}", index, local);
            }
            for block in &function.blocks {
                let _ = writeln!(out, "    block:{}", block.id.raw());
                for instr in &block.instructions {
                    let _ = writeln!(
                        out,
                        "      instr:{} span={} {}",
                        instr.id.raw(),
                        format_span(instr.span),
                        format_instruction(&instr.kind)
                    );
                }
                if let Some(terminator) = &block.terminator {
                    let _ = writeln!(
                        out,
                        "      term span={} {}",
                        format_span(terminator.span),
                        format_terminator(&terminator.kind)
                    );
                } else {
                    let _ = writeln!(out, "      term <missing>");
                }
            }
        }
        let _ = writeln!(out, "source_map:");
        for entry in self.source_map.entries() {
            let _ = writeln!(
                out,
                "  {} <= {} span={}",
                format_source_map_target(&entry.target),
                entry.origin,
                format_span(entry.span)
            );
        }
        out
    }

    /// Compatibility alias for early Prompt 03 tests and callers.
    #[must_use]
    pub fn to_debug_text(&self) -> String {
        self.to_snapshot_text()
    }

    /// Returns deterministic pretty JSON for IR snapshots and tools.
    pub fn to_json_pretty(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

impl fmt::Display for IrUnit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_snapshot_text())
    }
}

fn format_flags(top_level: bool, closure: bool, method: bool) -> String {
    let mut flags = Vec::new();
    if top_level {
        flags.push("top_level");
    }
    if closure {
        flags.push("closure");
    }
    if method {
        flags.push("method");
    }
    if flags.is_empty() {
        "none".to_string()
    } else {
        flags.join("|")
    }
}

fn format_return_type(return_type: &IrReturnType) -> String {
    match return_type {
        IrReturnType::Int => "int".to_string(),
        IrReturnType::Float => "float".to_string(),
        IrReturnType::String => "string".to_string(),
        IrReturnType::Array => "array".to_string(),
        IrReturnType::Callable => "callable".to_string(),
        IrReturnType::Object => "object".to_string(),
        IrReturnType::Bool => "bool".to_string(),
        IrReturnType::Null => "null".to_string(),
        IrReturnType::Void => "void".to_string(),
        IrReturnType::Mixed => "mixed".to_string(),
        IrReturnType::Class { name } => format!("class {name:?}"),
        IrReturnType::Nullable { inner } => format!("?{}", format_return_type(inner)),
    }
}

fn format_span(span: IrSpan) -> String {
    format!("file:{}@{}..{}", span.file.raw(), span.start, span.end)
}

fn format_constant(constant: &IrConstant) -> String {
    match constant {
        IrConstant::Null => "null".to_string(),
        IrConstant::Bool(value) => format!("bool {value}"),
        IrConstant::Int(value) => format!("int {value}"),
        IrConstant::Float(value) => format!("float {value:?}"),
        IrConstant::String(value) => format!("string {value:?}"),
    }
}

fn format_operand(operand: &Operand) -> String {
    match operand {
        Operand::Register(id) => format!("r{}", id.raw()),
        Operand::Local(id) => format!("local:{}", id.raw()),
        Operand::Constant(id) => format!("const:{}", id.raw()),
    }
}

fn format_unary_op(op: UnaryOp) -> &'static str {
    match op {
        UnaryOp::Plus => "plus",
        UnaryOp::Minus => "minus",
        UnaryOp::Not => "not",
        UnaryOp::BitNot => "bit_not",
    }
}

fn format_binary_op(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "add",
        BinaryOp::Sub => "sub",
        BinaryOp::Mul => "mul",
        BinaryOp::Div => "div",
        BinaryOp::Mod => "mod",
        BinaryOp::Concat => "concat",
        BinaryOp::Pow => "pow",
    }
}

fn format_compare_op(op: CompareOp) -> &'static str {
    match op {
        CompareOp::Equal => "equal",
        CompareOp::NotEqual => "not_equal",
        CompareOp::Identical => "identical",
        CompareOp::NotIdentical => "not_identical",
        CompareOp::Less => "less",
        CompareOp::LessEqual => "less_equal",
        CompareOp::Greater => "greater",
        CompareOp::GreaterEqual => "greater_equal",
        CompareOp::Spaceship => "spaceship",
    }
}

fn format_cast_kind(kind: CastKind) -> &'static str {
    match kind {
        CastKind::Bool => "bool",
        CastKind::Int => "int",
        CastKind::Float => "float",
        CastKind::String => "string",
        CastKind::Array => "array",
        CastKind::Object => "object",
        CastKind::Void => "void",
    }
}

fn format_instruction(kind: &InstructionKind) -> String {
    match kind {
        InstructionKind::Nop => "nop".to_string(),
        InstructionKind::LoadConst { dst, constant } => {
            format!("load_const r{} const:{}", dst.raw(), constant.raw())
        }
        InstructionKind::FetchConst { dst, name } => {
            format!("fetch_const r{} {:?}", dst.raw(), name)
        }
        InstructionKind::Move { dst, src } => {
            format!("move r{} {}", dst.raw(), format_operand(src))
        }
        InstructionKind::LoadLocal { dst, local } => {
            format!("load_local r{} local:{}", dst.raw(), local.raw())
        }
        InstructionKind::LoadLocalQuiet { dst, local } => {
            format!("load_local_quiet r{} local:{}", dst.raw(), local.raw())
        }
        InstructionKind::StoreLocal { local, src } => {
            format!("store_local local:{} {}", local.raw(), format_operand(src))
        }
        InstructionKind::BindReference { target, source } => {
            format!(
                "bind_reference local:{} local:{}",
                target.raw(),
                source.raw()
            )
        }
        InstructionKind::Binary { dst, op, lhs, rhs } => format!(
            "binary r{} {} {} {}",
            dst.raw(),
            format_binary_op(*op),
            format_operand(lhs),
            format_operand(rhs)
        ),
        InstructionKind::Compare { dst, op, lhs, rhs } => format!(
            "compare r{} {} {} {}",
            dst.raw(),
            format_compare_op(*op),
            format_operand(lhs),
            format_operand(rhs)
        ),
        InstructionKind::Unary { dst, op, src } => format!(
            "unary r{} {} {}",
            dst.raw(),
            format_unary_op(*op),
            format_operand(src)
        ),
        InstructionKind::Cast { dst, kind, src } => format!(
            "cast r{} {} {}",
            dst.raw(),
            format_cast_kind(*kind),
            format_operand(src)
        ),
        InstructionKind::Discard { src } => format!("discard {}", format_operand(src)),
        InstructionKind::Echo { src } => format!("echo {}", format_operand(src)),
        InstructionKind::CallFunction { dst, name, args } => {
            let args = args
                .iter()
                .map(format_operand)
                .collect::<Vec<_>>()
                .join(", ");
            format!("call_function r{} {:?} [{}]", dst.raw(), name, args)
        }
        InstructionKind::CallMethod {
            dst,
            object,
            method,
            args,
        } => {
            let args = args
                .iter()
                .map(format_operand)
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "call_method r{} {} {:?} [{}]",
                dst.raw(),
                format_operand(object),
                method,
                args
            )
        }
        InstructionKind::CallStaticMethod {
            dst,
            class_name,
            method,
            args,
        } => {
            let args = args
                .iter()
                .map(format_operand)
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "call_static_method r{} {:?}::{:?} [{}]",
                dst.raw(),
                class_name,
                method,
                args
            )
        }
        InstructionKind::CloneObject { dst, object } => {
            format!("clone_object r{} {}", dst.raw(), format_operand(object))
        }
        InstructionKind::CloneWith {
            dst,
            object,
            replacements,
        } => format!(
            "clone_with r{} {} {}",
            dst.raw(),
            format_operand(object),
            format_operand(replacements)
        ),
        InstructionKind::EnterTry {
            catch,
            finally,
            after,
            exception_local,
        } => format!(
            "enter_try catch={} finally={} after=block:{} exception_local={}",
            catch
                .map(|block| format!("block:{}", block.raw()))
                .unwrap_or_else(|| "none".to_owned()),
            finally
                .map(|block| format!("block:{}", block.raw()))
                .unwrap_or_else(|| "none".to_owned()),
            after.raw(),
            exception_local
                .map(|local| format!("local:{}", local.raw()))
                .unwrap_or_else(|| "none".to_owned())
        ),
        InstructionKind::LeaveTry => "leave_try".to_owned(),
        InstructionKind::EndFinally { after } => {
            format!("end_finally after=block:{}", after.raw())
        }
        InstructionKind::Throw { value } => format!("throw {}", format_operand(value)),
        InstructionKind::MakeException { dst, message } => {
            format!("make_exception r{} {}", dst.raw(), format_operand(message))
        }
        InstructionKind::MakeClosure {
            dst,
            function,
            captures,
        } => {
            let captures = captures
                .iter()
                .map(|capture| {
                    format!(
                        "{:?}={} by_ref={}",
                        capture.name,
                        format_operand(&capture.src),
                        capture.by_ref
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "make_closure r{} function:{} [{}]",
                dst.raw(),
                function.raw(),
                captures
            )
        }
        InstructionKind::CallClosure { dst, callee, args } => {
            let args = args
                .iter()
                .map(format_operand)
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "call_closure r{} {} [{}]",
                dst.raw(),
                format_operand(callee),
                args
            )
        }
        InstructionKind::ResolveCallable { dst, callable } => {
            format!(
                "resolve_callable r{} {}",
                dst.raw(),
                format_callable_kind(callable)
            )
        }
        InstructionKind::CallCallable { dst, callee, args } => {
            let args = args
                .iter()
                .map(format_operand)
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "call_callable r{} {} [{}]",
                dst.raw(),
                format_operand(callee),
                args
            )
        }
        InstructionKind::Pipe {
            dst,
            input,
            callable,
        } => format!(
            "pipe r{} {} {}",
            dst.raw(),
            format_operand(input),
            format_operand(callable)
        ),
        InstructionKind::Include { dst, kind, path } => format!(
            "include r{} {} {}",
            dst.raw(),
            format_include_kind(*kind),
            format_operand(path)
        ),
        InstructionKind::NewObject {
            dst,
            class_name,
            args,
        } => format!(
            "new_object r{} {:?} ({})",
            dst.raw(),
            class_name,
            args.iter()
                .map(format_operand)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        InstructionKind::FetchProperty {
            dst,
            object,
            property,
        } => format!(
            "fetch_property r{} {} ${}",
            dst.raw(),
            format_operand(object),
            property
        ),
        InstructionKind::AssignProperty {
            dst,
            object,
            property,
            value,
        } => format!(
            "assign_property r{} {} ${} {}",
            dst.raw(),
            format_operand(object),
            property,
            format_operand(value)
        ),
        InstructionKind::NewArray { dst } => format!("new_array r{}", dst.raw()),
        InstructionKind::ArrayInsert { array, key, value } => {
            let key = key
                .as_ref()
                .map(format_operand)
                .unwrap_or_else(|| "append".to_owned());
            format!(
                "array_insert r{} {} {}",
                array.raw(),
                key,
                format_operand(value)
            )
        }
        InstructionKind::FetchDim {
            dst,
            array,
            key,
            quiet,
        } => format!(
            "fetch_dim r{} {} {} quiet={}",
            dst.raw(),
            format_operand(array),
            format_operand(key),
            quiet
        ),
        InstructionKind::AssignDim {
            dst,
            local,
            dims,
            value,
        } => format!(
            "assign_dim r{} local:{} [{}] {}",
            dst.raw(),
            local.raw(),
            format_operands(dims),
            format_operand(value)
        ),
        InstructionKind::AppendDim {
            dst,
            local,
            dims,
            value,
        } => format!(
            "append_dim r{} local:{} [{}] {}",
            dst.raw(),
            local.raw(),
            format_operands(dims),
            format_operand(value)
        ),
        InstructionKind::IssetLocal { dst, local } => {
            format!("isset_local r{} local:{}", dst.raw(), local.raw())
        }
        InstructionKind::EmptyLocal { dst, local } => {
            format!("empty_local r{} local:{}", dst.raw(), local.raw())
        }
        InstructionKind::UnsetLocal { local } => format!("unset_local local:{}", local.raw()),
        InstructionKind::IssetDim { dst, local, dims } => format!(
            "isset_dim r{} local:{} [{}]",
            dst.raw(),
            local.raw(),
            format_operands(dims)
        ),
        InstructionKind::EmptyDim { dst, local, dims } => format!(
            "empty_dim r{} local:{} [{}]",
            dst.raw(),
            local.raw(),
            format_operands(dims)
        ),
        InstructionKind::UnsetDim { local, dims } => {
            format!(
                "unset_dim local:{} [{}]",
                local.raw(),
                format_operands(dims)
            )
        }
        InstructionKind::ForeachInit { iterator, source } => format!(
            "foreach_init iter:r{} source={}",
            iterator.raw(),
            format_operand(source)
        ),
        InstructionKind::ForeachNext {
            has_value,
            iterator,
            key,
            value,
        } => {
            let key = key
                .map(|key| format!("r{}", key.raw()))
                .unwrap_or_else(|| "none".to_string());
            format!(
                "foreach_next has=r{} iter:r{} key={} value=r{}",
                has_value.raw(),
                iterator.raw(),
                key,
                value.raw()
            )
        }
        InstructionKind::ArrayGet { dst, array, index } => format!(
            "array_get r{} {} {}",
            dst.raw(),
            format_operand(array),
            format_operand(index)
        ),
        InstructionKind::Unsupported { diagnostic_id } => {
            format!("unsupported {diagnostic_id:?}")
        }
        InstructionKind::RuntimeError {
            diagnostic_id,
            message,
        } => {
            format!("runtime_error {diagnostic_id:?} {message:?}")
        }
    }
}

fn format_operands(operands: &[Operand]) -> String {
    operands
        .iter()
        .map(format_operand)
        .collect::<Vec<_>>()
        .join(", ")
}

fn format_include_kind(kind: crate::instruction::IncludeKind) -> &'static str {
    match kind {
        crate::instruction::IncludeKind::Include => "include",
        crate::instruction::IncludeKind::IncludeOnce => "include_once",
        crate::instruction::IncludeKind::Require => "require",
        crate::instruction::IncludeKind::RequireOnce => "require_once",
    }
}

fn format_callable_kind(kind: &crate::instruction::CallableKind) -> String {
    match kind {
        crate::instruction::CallableKind::FunctionName { name } => {
            format!("function_name {:?}", name)
        }
        crate::instruction::CallableKind::MethodPlaceholder { target } => {
            format!("method_placeholder {:?}", target)
        }
        crate::instruction::CallableKind::UnresolvedDynamic { target } => {
            format!("unresolved_dynamic {:?}", target)
        }
    }
}

fn format_terminator(kind: &TerminatorKind) -> String {
    match kind {
        TerminatorKind::Jump { target } => format!("jump block:{}", target.raw()),
        TerminatorKind::JumpIfFalse { condition, target } => {
            format!(
                "jump_if_false {} block:{}",
                format_operand(condition),
                target.raw()
            )
        }
        TerminatorKind::JumpIfTrue { condition, target } => {
            format!(
                "jump_if_true {} block:{}",
                format_operand(condition),
                target.raw()
            )
        }
        TerminatorKind::JumpIf {
            condition,
            if_true,
            if_false,
        } => {
            format!(
                "jump_if {} block:{} block:{}",
                format_operand(condition),
                if_true.raw(),
                if_false.raw()
            )
        }
        TerminatorKind::Return { value } => match value {
            Some(value) => format!("return {}", format_operand(value)),
            None => "return".to_string(),
        },
    }
}

fn format_source_map_target(target: &IrSourceMapTarget) -> String {
    match target {
        IrSourceMapTarget::Function { function } => format!("function:{}", function.raw()),
        IrSourceMapTarget::Block { function, block } => {
            format!("block function:{} block:{}", function.raw(), block.raw())
        }
        IrSourceMapTarget::Instruction {
            function,
            block,
            instruction,
        } => format!(
            "instr function:{} block:{} instr:{}",
            function.raw(),
            block.raw(),
            instruction.raw()
        ),
        IrSourceMapTarget::Terminator { function, block } => {
            format!("term function:{} block:{}", function.raw(), block.raw())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::BasicBlock;
    use crate::function::{FunctionFlags, IrFunction};
    use crate::ids::{BlockId, ConstId, FileId, FunctionId, InstrId, LocalId, RegId, UnitId};
    use crate::instruction::{Instruction, Terminator};
    use crate::module::{FileEntry, IR_VERSION};

    #[test]
    fn display_covers_instruction_families() {
        let file = FileId::new(0);
        let span = IrSpan::new(file, 0, 1);
        let mut unit = IrUnit::new(UnitId::new(0));
        unit.files.push(FileEntry {
            id: file,
            path: "display.php".to_string(),
        });
        unit.constants.push(IrConstant::Int(1));
        let mut function = IrFunction::new(
            "display",
            FunctionFlags {
                is_top_level: true,
                ..FunctionFlags::default()
            },
            span,
        );
        function.local_count = 1;
        function.register_count = 3;
        let mut block = BasicBlock::new(BlockId::new(0));
        let instructions = [
            InstructionKind::Nop,
            InstructionKind::LoadConst {
                dst: RegId::new(0),
                constant: ConstId::new(0),
            },
            InstructionKind::FetchConst {
                dst: RegId::new(0),
                name: "ANSWER".to_string(),
            },
            InstructionKind::Move {
                dst: RegId::new(1),
                src: Operand::Register(RegId::new(0)),
            },
            InstructionKind::LoadLocal {
                dst: RegId::new(2),
                local: LocalId::new(0),
            },
            InstructionKind::LoadLocalQuiet {
                dst: RegId::new(2),
                local: LocalId::new(0),
            },
            InstructionKind::StoreLocal {
                local: LocalId::new(0),
                src: Operand::Register(RegId::new(2)),
            },
            InstructionKind::Binary {
                dst: RegId::new(0),
                op: BinaryOp::Add,
                lhs: Operand::Register(RegId::new(1)),
                rhs: Operand::Constant(ConstId::new(0)),
            },
            InstructionKind::Unary {
                dst: RegId::new(1),
                op: UnaryOp::Not,
                src: Operand::Register(RegId::new(0)),
            },
            InstructionKind::Cast {
                dst: RegId::new(2),
                kind: CastKind::String,
                src: Operand::Register(RegId::new(1)),
            },
            InstructionKind::Echo {
                src: Operand::Register(RegId::new(2)),
            },
            InstructionKind::Unsupported {
                diagnostic_id: "E_TEST_UNSUPPORTED".to_string(),
            },
        ];
        for (index, kind) in instructions.into_iter().enumerate() {
            block.instructions.push(Instruction {
                id: InstrId::new(index as u32),
                span,
                kind,
            });
        }
        block.terminator = Some(Terminator {
            span,
            kind: TerminatorKind::Return { value: None },
        });
        function.blocks.push(block);
        unit.functions.push(function);
        unit.entry = FunctionId::new(0);

        let text = unit.to_snapshot_text();
        for expected in [
            "nop",
            "load_const",
            "move",
            "load_local",
            "load_local_quiet",
            "store_local",
            "binary",
            "unary",
            "cast",
            "echo",
            "unsupported",
            "return",
        ] {
            assert!(text.contains(expected), "{expected} missing from {text}");
        }
        assert!(text.starts_with(&format!("ir version={IR_VERSION}")));
    }

    #[test]
    fn json_output_is_pretty_and_stable_enough_for_tools() {
        let unit = IrUnit::new(UnitId::new(9));
        let json = unit.to_json_pretty().expect("IR JSON should serialize");
        assert!(json.contains("\"version\": 1"));
        assert!(json.contains("\"id\""));
    }
}
