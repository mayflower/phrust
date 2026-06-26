--TEST--
Generated zend.functions: is_callable across callable forms
--DESCRIPTION--
module: zend.functions
generated timestamp: 20260626T000000Z
generator version: phpt-zend-functions-v1
reason: is_callable recognizes function-name strings, user functions, closures, [object, method] and [class, static-method] arrays, and rejects unknown targets (Zend/tests/closures/closure_001.phpt)
--FILE--
<?php
function my_fn() {}
class C {
    function m() {}
    static function s() {}
}
var_dump(is_callable("strlen"));
var_dump(is_callable("my_fn"));
var_dump(is_callable("no_such_function"));
var_dump(is_callable([new C(), "m"]));
var_dump(is_callable(["C", "s"]));
var_dump(is_callable([new C(), "nope"]));
var_dump(is_callable(function () {}));
var_dump(is_callable("no_such_function", true));
?>
--EXPECT--
bool(true)
bool(true)
bool(false)
bool(true)
bool(true)
bool(false)
bool(true)
bool(true)
