--TEST--
Generated zend.functions: callable type checks accept PHP callable forms
--DESCRIPTION--
module: zend.functions
generated timestamp: 20260627T000000Z
generator version: phpt-zend-functions-v1
reason: callable parameter and return types accept callable strings, closures, [class, static-method], and [object, method], while rejecting class strings for instance methods (Zend/tests/type_declarations/callable/callable_001.phpt and callable_003.phpt)
--FILE--
<?php
function accept(callable $callback): callable {
    return $callback;
}

function helper($value) {
    return $value;
}

class CallableTarget {
    public static function stat($value) {
        return $value;
    }

    public function inst($value) {
        return $value;
    }
}

$object = new CallableTarget();

var_dump(accept("strlen"));
var_dump(accept("helper"));
var_dump(accept(helper(...)) instanceof Closure);
var_dump(accept(["CallableTarget", "stat"]));
accept([$object, "inst"]);
echo "object-method\n";

function returns_callable(): callable {
    return "strlen";
}

var_dump(returns_callable());

try {
    accept(["CallableTarget", "inst"]);
} catch (TypeError $e) {
    echo "type-error\n";
}
?>
--EXPECT--
string(6) "strlen"
string(6) "helper"
bool(true)
array(2) {
  [0]=>
  string(14) "CallableTarget"
  [1]=>
  string(4) "stat"
}
object-method
string(6) "strlen"
type-error
