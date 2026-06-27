--TEST--
Generated zend.functions: ReflectionParameter::isCallable reports callable parameters
--DESCRIPTION--
module: zend.functions
generated timestamp: 20260627T000000Z
generator version: phpt-zend-functions-v1
reason: ReflectionParameter::isCallable remains a deprecated compatibility surface for callable parameter metadata
--FILE--
<?php
function prompt13_reflect_callable(callable $cb) {}

$rf = new ReflectionFunction("prompt13_reflect_callable");
var_dump($rf->getParameters()[0]->isCallable());
?>
--EXPECTF--
Deprecated: Method ReflectionParameter::isCallable() is deprecated since 8.0, use ReflectionParameter::getType() instead in %s on line %d
bool(true)
