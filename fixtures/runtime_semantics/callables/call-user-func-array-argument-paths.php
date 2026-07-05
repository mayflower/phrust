<?php
// call_user_func_array argument handling: packed positional args (owned
// temporaries and shared locals), string keys as named args, by-value
// isolation, callback shapes, and invalid callbacks.

function sum3($a, $b, $c) { return $a + $b + $c; }
function tail(...$rest) { return implode(",", $rest); }
function greet($name, $greeting = "hi") { return "$greeting $name"; }

class Adder {
    public $base;
    public function __construct($base) { $this->base = $base; }
    public function add($x) { return $this->base + $x; }
    public static function twice($x) { return $x * 2; }
}

// Owned temporary argument array (hook-dispatch shape).
echo call_user_func_array('sum3', array_slice([1, 2, 3, 4], 0, 3)), "\n";

// Shared local argument array must not be consumed or mutated.
$args = [10, 20, 30];
echo call_user_func_array('sum3', $args), "\n";
echo count($args), ":", $args[0], "\n";

// Array values stay by-value: callee mutation is invisible.
function mutate_first(array $items) { $items[0] = 99; return $items[0]; }
$shared = [1, 2];
echo call_user_func_array('mutate_first', [$shared]), ":", $shared[0], "\n";

// Named (string-key) arguments.
echo call_user_func_array('greet', ['name' => 'ada', 'greeting' => 'hello']), "\n";

// Variadics through owned arrays.
echo call_user_func_array('tail', ['a', 'b', 'c']), "\n";

// Closure / method / static-method callbacks.
$double = fn($x) => $x * 2;
echo call_user_func_array($double, [21]), "\n";
$adder = new Adder(5);
echo call_user_func_array([$adder, 'add'], [7]), "\n";
echo call_user_func_array('Adder::twice', [8]), "\n";

// NOTE: invalid callbacks through call_user_func_array currently raise an
// uncatchable engine fatal (pre-existing E_PHP_VM_UNRESOLVED_CALLABLE gap);
// pinned elsewhere once catchable TypeError routing lands.
