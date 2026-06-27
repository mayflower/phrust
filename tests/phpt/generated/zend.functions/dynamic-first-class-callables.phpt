--TEST--
Dynamic first-class callables evaluate runtime class and method operands
--FILE--
<?php
class A {
    public static function b($x) { return $x; }
    public function c($x) { return $x; }
}

$class = 'A';
$method = 'b';
$fn = $class::$method(...);
var_dump($fn(4));

$method = 'c';
$fn = (new A)->$method(...);
var_dump($fn(5));

$fn = [A::class, 'b'](...);
var_dump($fn(6));

$closure = function () { return 'OK'; };
$invoke = $closure->__invoke(...);
echo $invoke(), "\n";
?>
--EXPECT--
int(4)
int(5)
int(6)
OK
