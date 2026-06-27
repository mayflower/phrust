--TEST--
Generated zend.functions: is_callable supports callable_name named out parameter
--DESCRIPTION--
module: zend.functions
generated timestamp: 20260627T000000Z
generator version: phpt-zend-functions-v1
reason: is_callable accepts named callable_name and writes the resolved callable name for first-class callables (Zend/tests/first_class_callable/gh18062.phpt)
--FILE--
<?php
class CallableNameTarget {
    public function __invoke() {}
    public static function stat() {}
}

function callable_name_helper() {}

is_callable(callable_name_helper(...), callable_name: $name);
var_dump($name);

is_callable((new CallableNameTarget())(...), callable_name: $name);
var_dump($name);

is_callable(CallableNameTarget::stat(...), callable_name: $name);
var_dump($name);
?>
--EXPECT--
string(20) "callable_name_helper"
string(28) "CallableNameTarget::__invoke"
string(24) "CallableNameTarget::stat"
