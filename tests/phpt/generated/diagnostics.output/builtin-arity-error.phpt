--TEST--
Generated smoke: builtin arity error is catchable ArgumentCountError
--DESCRIPTION--
reference behavior: PHP 8.5.7 CLI builtin arity diagnostic
generated timestamp: 20260625T000000Z
generator version: phpt-diagnostics-output-v1
reason: central builtin diagnostic mapping for arity failures
--FILE--
<?php
try {
    strlen();
} catch (ArgumentCountError $e) {
    echo "arity\n";
}
--EXPECT--
arity
