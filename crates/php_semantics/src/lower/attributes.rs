//! Structural attribute lowering.

use std::collections::HashSet;

use crate::FrontendDatabase;
use crate::hir::{
    AttributeTarget, ExprId, HirAttribute, HirNameResolution, ModuleId, QualifiedName,
};
use crate::lower::types::TypeLoweringScope;
use crate::symbols::resolution::{ResolveContext, ResolvedName};
use php_ast::{
    AnonymousClassExpr, AstNode, AstToken, Attribute, AttributeGroup, ClassConstDecl, ClassDecl,
    ClosureExpr, EnumCase, EnumDecl, ExprNode, FunctionDecl, InterfaceDecl, MethodDecl, Name,
    Param, PropertyDecl, TraitDecl, descendant_nodes, syntax_child_nodes, syntax_child_tokens,
};
use php_source::TextRange;
use php_syntax::SyntaxNode;

/// Collects attributes visible inside one top-level node.
pub fn collect_attributes_in_node(
    node: &SyntaxNode,
    database: &mut FrontendDatabase,
    module_id: ModuleId,
    scope: TypeLoweringScope,
) {
    let mut collector = AttributeCollector {
        database,
        module_id,
        scope,
    };
    collector.collect_node(node);
}

struct AttributeCollector<'a> {
    database: &'a mut FrontendDatabase,
    module_id: ModuleId,
    scope: TypeLoweringScope,
}

impl AttributeCollector<'_> {
    fn collect_node(&mut self, node: &SyntaxNode) {
        if ClassDecl::cast(node).is_some() {
            self.collect_direct_attributes(node, AttributeTarget::Class);
        } else if InterfaceDecl::cast(node).is_some() {
            self.collect_direct_attributes(node, AttributeTarget::Interface);
        } else if TraitDecl::cast(node).is_some() {
            self.collect_direct_attributes(node, AttributeTarget::Trait);
        } else if EnumDecl::cast(node).is_some() {
            self.collect_direct_attributes(node, AttributeTarget::Enum);
        } else if FunctionDecl::cast(node).is_some() {
            self.collect_direct_attributes(node, AttributeTarget::Function);
        } else if MethodDecl::cast(node).is_some() {
            self.collect_direct_attributes(node, AttributeTarget::Method);
        } else if ClassConstDecl::cast(node).is_some() {
            self.collect_direct_attributes(node, AttributeTarget::ClassConstant);
        } else if EnumCase::cast(node).is_some() {
            self.collect_direct_attributes(node, AttributeTarget::EnumCase);
        } else if PropertyDecl::cast(node).is_some() {
            self.collect_direct_attributes(node, AttributeTarget::Property);
        } else if Param::cast(node).is_some() {
            self.collect_direct_attributes(node, AttributeTarget::Parameter);
        } else if ClosureExpr::cast(node).is_some() {
            self.collect_direct_attributes(node, AttributeTarget::Closure);
        } else if AnonymousClassExpr::cast(node).is_some() {
            self.collect_direct_attributes(node, AttributeTarget::AnonymousClass);
        }

        for child in syntax_child_nodes(node) {
            self.collect_node(child);
        }
    }

    fn collect_direct_attributes(&mut self, target_node: &SyntaxNode, target: AttributeTarget) {
        let mut seen = HashSet::new();
        for group in syntax_child_nodes(target_node).filter_map(AttributeGroup::cast) {
            for attribute in descendant_nodes::<Attribute<'_>>(group.syntax()) {
                let name = self.resolve_attribute_name(attribute.syntax());
                let repeated_on_target = !seen.insert(name.source().to_ascii_lowercase());
                let args = self.attribute_args(attribute.syntax());
                let id = self
                    .database
                    .module_mut(self.module_id)
                    .expect("module allocated before attribute lowering")
                    .attributes_mut()
                    .alloc(HirAttribute::new(target, name, args, repeated_on_target));
                self.database
                    .source_map_mut()
                    .insert(id, attribute.text_range());
            }
        }
    }

    fn resolve_attribute_name(&self, node: &SyntaxNode) -> HirNameResolution {
        let source = syntax_child_nodes(node)
            .find(|child| Name::cast(child).is_some())
            .map(source_text_no_trivia)
            .unwrap_or_default();
        let qualified = QualifiedName::parse(&source);
        let result = self
            .scope
            .resolver()
            .resolve(&qualified, ResolveContext::AttributeClass);
        let name_kind =
            crate::symbols::resolution::NameResolver::name_kind(ResolveContext::AttributeClass);
        let resolved = match &result {
            ResolvedName::FullyQualified(name) => Some(name.canonical(name_kind)),
            ResolvedName::MaybeRuntimeFallback { namespaced, .. } => {
                Some(namespaced.canonical(name_kind))
            }
            ResolvedName::Dynamic | ResolvedName::Unresolved => None,
        };
        HirNameResolution::new(
            source,
            ResolveContext::AttributeClass.as_str(),
            result.classification(),
            resolved,
            None,
        )
    }

    fn attribute_args(&self, node: &SyntaxNode) -> Vec<ExprId> {
        let argument_start = syntax_child_tokens(node)
            .find(|token| token.text() == "(")
            .map(|token| token.text_range().end().to_usize());
        syntax_child_nodes(node)
            .filter(|child| {
                ExprNode::cast(child).is_some()
                    && argument_start
                        .is_some_and(|start| child.text_range().start().to_usize() >= start)
            })
            .filter_map(|expr| self.expr_id_for_node(expr))
            .collect()
    }

    fn expr_id_for_node(&self, node: &SyntaxNode) -> Option<ExprId> {
        self.expr_id_for_span(node.text_range()).or_else(|| {
            if node.kind().name() != "EXPR" {
                return None;
            }
            syntax_child_nodes(node)
                .filter(|child| ExprNode::cast(child).is_some())
                .last()
                .and_then(|child| self.expr_id_for_node(child))
        })
    }

    fn expr_id_for_span(&self, span: TextRange) -> Option<ExprId> {
        let module = self
            .database
            .module(self.module_id)
            .expect("module allocated before attribute lowering");
        module
            .expressions()
            .iter()
            .find_map(|(id, _)| (self.database.source_map().span(id) == Some(span)).then_some(id))
    }
}

fn source_text_no_trivia(node: &SyntaxNode) -> String {
    php_ast::descendant_tokens::<php_ast::TokenView<'_>>(node)
        .filter(|token| !token.kind().is_trivia())
        .map(|token| token.text())
        .collect::<Vec<_>>()
        .join("")
}
