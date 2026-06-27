--TEST--
Generated zend.functions: array_map validates callback before iteration
--DESCRIPTION--
module: zend.functions
generated timestamp: 20260627T000000Z
generator version: phpt-zend-functions-v1
reason: array_map callback validation is PHP-visible even when input arrays are empty (Zend/tests/type_declarations/internal_function_strict_mode.phpt)
--FILE--
<?php
try {
    array_map([null, "bar"], []);
} catch (TypeError $e) {
    echo $e->getMessage(), "\n";
}
?>
--EXPECT--
array_map(): Argument #1 ($callback) must be a valid callback or null, first array member is not a valid class name or object
