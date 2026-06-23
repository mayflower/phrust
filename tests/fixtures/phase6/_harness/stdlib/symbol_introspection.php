<?php
// phase6-diff: id=PHASE6_STDLIB_SYMBOL_INTROSPECTION area=stdlib expect=pass
const LOCAL_CONST = 41;
function local_fn() {}
interface I {}
class A { public $x; public function m() {} }
class B extends A implements I {}
enum E { case One; }
$b = new B();
echo defined('LOCAL_CONST') ? "D\n" : "d\n";
echo constant('LOCAL_CONST'), "\n";
echo function_exists('LOCAL_FN') ? "F\n" : "f\n";
echo class_exists('b', false) ? "C\n" : "c\n";
echo interface_exists('i', false) ? "I\n" : "i\n";
echo enum_exists('e', false) ? "E\n" : "e\n";
echo method_exists('B', 'M') ? "M\n" : "m\n";
echo property_exists('B', 'x') ? "P\n" : "p\n";
echo is_subclass_of('B', 'A') ? "S\n" : "s\n";
echo is_subclass_of('B', 'A', false) ? "bad\n" : "N\n";
echo get_class($b), "\n";
echo get_parent_class('B'), "\n";
echo in_array('B', get_declared_classes(), true) ? "DC\n" : "dc\n";
echo in_array('I', get_declared_interfaces(), true) ? "DI\n" : "di\n";
