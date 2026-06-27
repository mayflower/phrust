--TEST--
Generated zend.functions: Closure is a runtime class for callables
--DESCRIPTION--
module: zend.functions
generated timestamp: 20260627T000000Z
generator version: phpt-zend-functions-v1
reason: first-class callables are Closure instances for class lookup, instanceof, type checks, and direct-instantiation errors (Zend/tests/closures/closure_instantiate.phpt and Zend/tests/closures/closure_061.phpt)
--FILE--
<?php
function takesClosure(Closure $f): object {
    return $f;
}

$f = strlen(...);
echo class_exists("Closure") ? "class\n" : "missing\n";
echo ($f instanceof Closure) ? "instance\n" : "not-instance\n";
echo (takesClosure($f))("abcd"), "\n";
try {
    new Closure();
} catch (Exception $e) {
    echo "exception:", $e->getMessage(), "\n";
} catch (Throwable $e) {
    echo "error:", $e->getMessage(), "\n";
}
?>
--EXPECT--
class
instance
4
error:Instantiation of class Closure is not allowed
