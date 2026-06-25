--TEST--
Generated smoke: builtin type error is catchable TypeError
--DESCRIPTION--
reference behavior: PHP 8.5.7 CLI builtin type diagnostic
generated timestamp: 20260625T000000Z
generator version: phpt-diagnostics-output-v1
reason: central builtin diagnostic mapping for type failures
--FILE--
<?php
try {
    strlen([]);
} catch (TypeError $e) {
    echo "type\n";
}
--EXPECT--
type
