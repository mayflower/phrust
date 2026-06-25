--TEST--
Generated smoke: invalid array operand is catchable TypeError
--DESCRIPTION--
reference behavior: PHP 8.5.7 CLI unsupported operand diagnostic
generated timestamp: 20260625T000000Z
generator version: phpt-diagnostics-output-v1
reason: central fatal diagnostic channel maps invalid operands to TypeError
--FILE--
<?php
try {
    [] - [];
} catch (TypeError $e) {
    echo "invalid:", $e->getMessage(), "\n";
}
--EXPECT--
invalid:Unsupported operand types: array - array
