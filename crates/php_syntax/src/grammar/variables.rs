//! Variable grammar.

use crate::grammar::{named, symbol};
use crate::parser::core::Parser;
use crate::{SyntaxKind, SyntaxNodeKind};
use php_lexer::TokenName;

/// Parses a simple variable such as `$x`.
pub(crate) fn parse_simple_variable(parser: &mut Parser<'_>) -> bool {
    if !parser.at(named(TokenName::Variable))
        && !(parser.at(symbol(b'$')) && parser.nth(1) == symbol(b'{'))
    {
        return false;
    }

    let variable = parser.start();
    if parser.at(named(TokenName::Variable)) {
        parser.bump();
    } else {
        parser.bump();
        bump_trivia(parser);
        if !parser.at(symbol(b'{')) {
            parser.error_expected("expected `{` after `$` in braced variable", &["{"]);
        } else {
            parser.bump();
        }
        bump_trivia(parser);
        if !parser.at(named(TokenName::LNumber)) {
            parser.error_expected("expected integer braced variable name", &["T_LNUMBER"]);
        } else {
            parser.bump();
        }
        bump_trivia(parser);
        if parser.at(symbol(b'}')) {
            parser.bump();
        } else {
            parser.error_expected("expected `}` after braced variable name", &["}"]);
        }
    }
    let _completed = variable.complete(parser, SyntaxKind::Node(SyntaxNodeKind::Variable));
    true
}

fn bump_trivia(parser: &mut Parser<'_>) {
    while parser.current().is_trivia() {
        parser.bump();
    }
}
