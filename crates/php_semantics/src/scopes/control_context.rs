//! Compile-time control-context tracking for Phase 3.

use crate::diagnostics::{
    DiagnosticId, DiagnosticPhase, DiagnosticReporter, DiagnosticSeverity, SemanticDiagnostic,
};
use php_ast::{
    ArrowFunctionExpr, AstNode, AstToken, BreakStmt, ClassDecl, ClosureExpr, ContinueStmt,
    DoWhileStmt, EnumDecl, FinallyClause, ForStmt, ForeachStmt, FunctionDecl, GotoStmt,
    InterfaceDecl, LabelStmt, MatchExpr, MethodDecl, ReturnStmt, SwitchStmt, TokenView, TraitDecl,
    WhileStmt, YieldExpr, descendant_tokens, syntax_child_nodes, syntax_child_tokens,
};
use php_source::TextRange;
use php_syntax::SyntaxNode;
use std::collections::HashSet;

/// Stack of compile-time control contexts.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ControlContextStack {
    frames: Vec<ControlContextKind>,
}

impl ControlContextStack {
    /// Creates an empty control-context stack.
    #[must_use]
    pub const fn new() -> Self {
        Self { frames: Vec::new() }
    }

    /// Pushes a context.
    pub fn push(&mut self, kind: ControlContextKind) {
        self.frames.push(kind);
    }

    /// Pops the current context.
    pub fn pop(&mut self) -> Option<ControlContextKind> {
        self.frames.pop()
    }

    /// Runs a closure with a context pushed.
    pub fn with_context<T>(
        &mut self,
        kind: ControlContextKind,
        f: impl FnOnce(&mut Self) -> T,
    ) -> T {
        self.push(kind);
        let result = f(self);
        self.pop();
        result
    }

    /// Counts loop/switch targets visible to `break` and `continue`.
    ///
    /// Counting stops at the nearest function-like boundary. A closure inside a
    /// loop cannot use `break` to target the enclosing loop.
    #[must_use]
    pub fn break_continue_target_depth(&self) -> usize {
        self.frames
            .iter()
            .rev()
            .take_while(|kind| **kind != ControlContextKind::FunctionLike)
            .filter(|kind| matches!(kind, ControlContextKind::Loop | ControlContextKind::Switch))
            .count()
    }

    /// Returns true when currently inside a function-like context.
    #[must_use]
    pub fn in_function_like(&self) -> bool {
        self.frames
            .iter()
            .rev()
            .any(|kind| *kind == ControlContextKind::FunctionLike)
    }

    /// Returns true when top-level file control is visible.
    #[must_use]
    pub fn in_global_file(&self) -> bool {
        self.frames.contains(&ControlContextKind::GlobalFile)
    }

    /// Returns true when currently inside a class-like body but not a method.
    #[must_use]
    pub fn in_class_like_body_without_function(&self) -> bool {
        !self.in_function_like() && self.frames.contains(&ControlContextKind::ClassLikeBody)
    }
}

/// Compile-time control context kinds.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ControlContextKind {
    /// Loop construct: `while`, `do`, `for`, or `foreach`.
    Loop,
    /// `switch` construct.
    Switch,
    /// Function, method, closure, or arrow function body.
    FunctionLike,
    /// `finally` clause.
    Finally,
    /// Match expression or arm body. Current CST has no separate match-arm node.
    MatchArm,
    /// Class, trait, interface, or enum body.
    ClassLikeBody,
    /// Whole PHP file / include body.
    GlobalFile,
}

/// Runs Phase 3 control-context validation over a source file syntax tree.
#[must_use]
pub fn check_source_file(source_file: &SyntaxNode) -> Vec<SemanticDiagnostic> {
    let mut checker = ControlContextChecker::new();
    checker.stack.push(ControlContextKind::GlobalFile);
    checker.walk_children(source_file);
    checker.finish_current_unit();
    checker.reporter.into_diagnostics()
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct GotoUse {
    label: String,
    span: TextRange,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct ControlUnit {
    labels: HashSet<String>,
    gotos: Vec<GotoUse>,
}

struct ControlContextChecker {
    reporter: DiagnosticReporter,
    stack: ControlContextStack,
    units: Vec<ControlUnit>,
    unit_stack: Vec<usize>,
    function_stack: Vec<FunctionReturnContext>,
}

impl ControlContextChecker {
    fn new() -> Self {
        Self {
            reporter: DiagnosticReporter::new(),
            stack: ControlContextStack::new(),
            units: vec![ControlUnit::default()],
            unit_stack: vec![0],
            function_stack: Vec::new(),
        }
    }

    fn walk_node(&mut self, node: &SyntaxNode) {
        if FunctionDecl::cast(node).is_some()
            || MethodDecl::cast(node).is_some()
            || ClosureExpr::cast(node).is_some()
            || ArrowFunctionExpr::cast(node).is_some()
        {
            self.with_function_like_unit(node);
        } else if ClassDecl::cast(node).is_some()
            || InterfaceDecl::cast(node).is_some()
            || TraitDecl::cast(node).is_some()
            || EnumDecl::cast(node).is_some()
        {
            self.with_context(ControlContextKind::ClassLikeBody, node);
        } else if WhileStmt::cast(node).is_some()
            || DoWhileStmt::cast(node).is_some()
            || ForStmt::cast(node).is_some()
            || ForeachStmt::cast(node).is_some()
        {
            self.with_context(ControlContextKind::Loop, node);
        } else if SwitchStmt::cast(node).is_some() {
            self.with_context(ControlContextKind::Switch, node);
        } else if FinallyClause::cast(node).is_some() {
            self.with_context(ControlContextKind::Finally, node);
        } else if MatchExpr::cast(node).is_some() {
            self.with_context(ControlContextKind::MatchArm, node);
        } else if let Some(break_stmt) = BreakStmt::cast(node) {
            self.check_break_continue(
                break_stmt.syntax(),
                DiagnosticId::BreakNotInLoopOrSwitch,
                "break statement is not inside a loop or switch",
            );
        } else if let Some(continue_stmt) = ContinueStmt::cast(node) {
            self.check_break_continue(
                continue_stmt.syntax(),
                DiagnosticId::ContinueNotInLoopOrSwitch,
                "continue statement is not inside a loop or switch",
            );
        } else if let Some(return_stmt) = ReturnStmt::cast(node) {
            self.check_return(return_stmt);
        } else if let Some(yield_expr) = YieldExpr::cast(node) {
            self.check_yield(yield_expr);
        } else if let Some(goto_stmt) = GotoStmt::cast(node) {
            self.record_goto(goto_stmt);
        } else if let Some(label_stmt) = LabelStmt::cast(node) {
            self.record_label(label_stmt);
        } else {
            self.walk_children(node);
        }
    }

    fn walk_children(&mut self, node: &SyntaxNode) {
        for child in syntax_child_nodes(node) {
            self.walk_node(child);
        }
    }

    fn with_context(&mut self, kind: ControlContextKind, node: &SyntaxNode) {
        self.stack.push(kind);
        self.walk_children(node);
        self.stack.pop();
    }

    fn with_function_like_unit(&mut self, node: &SyntaxNode) {
        self.stack.push(ControlContextKind::FunctionLike);
        self.function_stack.push(function_return_context(node));
        let unit_id = self.units.len();
        self.units.push(ControlUnit::default());
        self.unit_stack.push(unit_id);
        self.walk_children(node);
        self.finish_current_unit();
        self.unit_stack.pop();
        self.function_stack.pop();
        self.stack.pop();
    }

    fn check_break_continue(
        &mut self,
        node: &SyntaxNode,
        outside_id: DiagnosticId,
        outside_message: &'static str,
    ) {
        let depth = self.stack.break_continue_target_depth();
        let level = break_continue_level(node).unwrap_or(1);
        if depth == 0 {
            self.reporter.error(
                outside_id,
                DiagnosticPhase::ControlFlowValidation,
                outside_message,
                Some(node.text_range()),
            );
        } else if level == 0 || level > depth {
            self.reporter.error(
                DiagnosticId::InvalidBreakContinueLevel,
                DiagnosticPhase::ControlFlowValidation,
                format!(
                    "break/continue level {level} exceeds {depth} visible loop/switch context(s)"
                ),
                Some(node.text_range()),
            );
        }
    }

    fn check_return(&mut self, return_stmt: ReturnStmt<'_>) {
        if self.stack.in_class_like_body_without_function()
            || (!self.stack.in_function_like() && !self.stack.in_global_file())
        {
            self.reporter.error(
                DiagnosticId::ReturnOutsideAllowedContext,
                DiagnosticPhase::ControlFlowValidation,
                "return statement is outside an allowed PHP file or function-like context",
                Some(return_stmt.text_range()),
            );
            return;
        }

        let Some(function) = self.function_stack.last().copied() else {
            return;
        };
        if function.returns_void && return_has_value(return_stmt.syntax()) {
            self.reporter.error(
                DiagnosticId::ReturnValueFromVoidFunction,
                DiagnosticPhase::ControlFlowValidation,
                "void function must not return a value",
                Some(return_stmt.text_range()),
            );
        }
        if function.returns_never {
            self.reporter.error(
                DiagnosticId::ReturnFromNeverFunction,
                DiagnosticPhase::ControlFlowValidation,
                "never-returning function must not contain return statements",
                Some(return_stmt.text_range()),
            );
        }
    }

    fn check_yield(&mut self, yield_expr: YieldExpr<'_>) {
        if !self.stack.in_function_like() {
            self.reporter.error(
                DiagnosticId::YieldOutsideFunction,
                DiagnosticPhase::ControlFlowValidation,
                "yield expression is outside a function-like context",
                Some(yield_expr.text_range()),
            );
        }
    }

    fn record_goto(&mut self, goto_stmt: GotoStmt<'_>) {
        if let Some((label, span)) = label_name(goto_stmt.syntax()) {
            let current = self.current_unit_mut();
            current.gotos.push(GotoUse { label, span });
        }
    }

    fn record_label(&mut self, label_stmt: LabelStmt<'_>) {
        if let Some((label, _)) = label_name(label_stmt.syntax()) {
            self.current_unit_mut().labels.insert(label);
        }
    }

    fn current_unit_mut(&mut self) -> &mut ControlUnit {
        let unit_id = *self.unit_stack.last().expect("global unit exists");
        self.units.get_mut(unit_id).expect("current unit exists")
    }

    fn finish_current_unit(&mut self) {
        let unit_id = *self.unit_stack.last().expect("global unit exists");
        let unit = self.units.get(unit_id).expect("current unit exists");
        for goto_use in &unit.gotos {
            if !unit.labels.contains(&goto_use.label) {
                self.reporter.report(SemanticDiagnostic::with_span(
                    DiagnosticId::GotoLabelNotFound,
                    DiagnosticSeverity::Error,
                    DiagnosticPhase::ControlFlowValidation,
                    format!("goto label `{}` was not found", goto_use.label),
                    goto_use.span,
                ));
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct FunctionReturnContext {
    returns_void: bool,
    returns_never: bool,
}

fn function_return_context(node: &SyntaxNode) -> FunctionReturnContext {
    let mut after_colon = false;
    let mut context = FunctionReturnContext::default();
    for token in descendant_tokens::<TokenView<'_>>(node).filter(|token| !token.kind().is_trivia())
    {
        match token.text() {
            ":" => after_colon = true,
            "{" | ";" | "=>" if after_colon => break,
            _ if after_colon
                && token_matches_kind_or_text(&token.kind().name(), token.text(), "T_VOID") =>
            {
                context.returns_void = true;
            }
            _ if after_colon
                && token_matches_kind_or_text(&token.kind().name(), token.text(), "T_NEVER") =>
            {
                context.returns_never = true;
            }
            _ => {}
        }
    }
    context
}

fn token_matches_kind_or_text(kind: &str, text: &str, token_name: &str) -> bool {
    kind == token_name
        || match token_name {
            "T_VOID" => text.eq_ignore_ascii_case("void"),
            "T_NEVER" => text.eq_ignore_ascii_case("never"),
            _ => false,
        }
}

fn return_has_value(node: &SyntaxNode) -> bool {
    syntax_child_nodes(node).any(|child| child.kind().name() == "EXPR")
        || syntax_child_tokens(node)
            .filter(|token| !token.kind().is_trivia())
            .skip_while(|token| token.kind().name() == "T_RETURN")
            .any(|token| token.text() != ";")
}

fn break_continue_level(node: &SyntaxNode) -> Option<usize> {
    descendant_tokens::<TokenView<'_>>(node)
        .find(|token| token.kind().name() == "T_LNUMBER")
        .and_then(|token| token.text().parse::<usize>().ok())
}

fn label_name(node: &SyntaxNode) -> Option<(String, TextRange)> {
    syntax_child_tokens(node)
        .find(|token| token.kind().name() == "T_STRING")
        .map(|token| (token.text().to_owned(), token.text_range()))
}

#[cfg(test)]
mod tests {
    use super::{ControlContextKind, ControlContextStack, check_source_file};
    use crate::diagnostics::DiagnosticId;
    use php_ast::{AstNode, source_file};
    use php_syntax::parse_source_file;

    #[test]
    fn break_depth_stops_at_function_like_boundary() {
        let mut stack = ControlContextStack::new();
        stack.push(ControlContextKind::GlobalFile);
        stack.push(ControlContextKind::Loop);
        assert_eq!(stack.break_continue_target_depth(), 1);

        stack.push(ControlContextKind::FunctionLike);
        assert_eq!(stack.break_continue_target_depth(), 0);
    }

    #[test]
    fn diagnoses_break_and_continue_outside_loop_or_switch() {
        let parse = parse_source_file("<?php break; continue;");
        let root = source_file(parse.root()).expect("source file");
        let diagnostics = check_source_file(root.syntax());

        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id() == DiagnosticId::BreakNotInLoopOrSwitch)
        );
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id() == DiagnosticId::ContinueNotInLoopOrSwitch)
        );
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.span().is_some())
        );
    }

    #[test]
    fn diagnoses_invalid_break_continue_level() {
        let parse = parse_source_file("<?php while (true) { break 2; }");
        let root = source_file(parse.root()).expect("source file");
        let diagnostics = check_source_file(root.syntax());

        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id() == DiagnosticId::InvalidBreakContinueLevel)
        );
    }

    #[test]
    fn allows_top_level_return_and_function_yield() {
        let parse = parse_source_file("<?php return 1; function gen() { yield 1; }");
        let root = source_file(parse.root()).expect("source file");
        let diagnostics = check_source_file(root.syntax());

        assert!(diagnostics.is_empty());
    }

    #[test]
    fn diagnoses_top_level_yield_and_missing_goto_label() {
        let parse = parse_source_file("<?php yield 1; goto missing;");
        let root = source_file(parse.root()).expect("source file");
        let diagnostics = check_source_file(root.syntax());

        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id() == DiagnosticId::YieldOutsideFunction)
        );
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id() == DiagnosticId::GotoLabelNotFound)
        );
    }

    #[test]
    fn diagnoses_return_value_from_void_and_return_from_never() {
        let parse = parse_source_file(
            "<?php function bad_void(): void { return 1; } function bad_never(): never { return; }",
        );
        let root = source_file(parse.root()).expect("source file");
        let diagnostics = check_source_file(root.syntax());

        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id() == DiagnosticId::ReturnValueFromVoidFunction)
        );
        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.id() == DiagnosticId::ReturnFromNeverFunction)
        );
    }
}
