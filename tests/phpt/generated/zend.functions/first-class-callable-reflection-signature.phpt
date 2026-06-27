--TEST--
Generated zend.functions: first-class callables retain signatures for ReflectionFunction
--DESCRIPTION--
module: zend.functions
generated timestamp: 20260627T000000Z
generator version: phpt-zend-functions-v1
reason: first-class callables expose retained user-function signatures through ReflectionFunction string output
--FILE--
<?php
function prompt13_reflect_signature(int $a, string &$b, Prompt13ReflectSignatureFoo ...$c) {}

echo new ReflectionFunction(prompt13_reflect_signature(...));
?>
--EXPECTF--
Closure [ <user> function prompt13_reflect_signature ] {
  @@ %s %d - %d

  - Parameters [3] {
    Parameter #0 [ <required> int $a ]
    Parameter #1 [ <required> string &$b ]
    Parameter #2 [ <optional> Prompt13ReflectSignatureFoo ...$c ]
  }
}
