<?php
// runtime-semantics: category=callables expect=pass php_ref_required=1
// Callable calls through dense execution: closures with by-value and
// by-ref captures, callable strings, static-method strings, invokable
// objects, callable arrays, exceptions from closure bodies, undefined
// callables, and nested higher-order dispatch.

function apply_twice($fn, $x) {
    return $fn($fn($x));
}

$double = function ($n) {
    return $n * 2;
};
$sum = 0;
for ($i = 1; $i <= 4; $i++) {
    $sum += apply_twice($double, $i);
}
echo $sum, "\n";

// Builtin and userland callable strings resolve per call site.
echo apply_twice('strrev', 'abc'), "|", apply_twice('trim', "  x  "), "\n";
function increment($n) {
    return $n + 1;
}
echo apply_twice('increment', 10), "\n";

// Captures: by value snapshots, by reference observes and writes back.
$base = 100;
$plus_base = fn($n) => $n + $base;
$base = 200;
echo $plus_base(1), "\n";
$count = 0;
$tally = function () use (&$count) {
    $count++;
    return $count;
};
echo $tally(), $tally(), $tally(), "|", $count, "\n";

// Static-method callable string and callable array.
class MathOps {
    public static function square($n) {
        return $n * $n;
    }
    public function cube($n) {
        return $n * $n * $n;
    }
}
echo apply_twice('MathOps::square', 2), "\n";
$ops = new MathOps();
echo apply_twice([$ops, 'cube'], 2), "\n";
echo apply_twice(['MathOps', 'square'], 3), "\n";

// Invokable object through the callable path.
class Adder {
    private $step;
    public function __construct($step) {
        $this->step = $step;
    }
    public function __invoke($n) {
        return $n + $this->step;
    }
}
echo apply_twice(new Adder(5), 0), "\n";

// First-class callable syntax stays on the rich plan (resolve_callable
// is a per-function local fallback, not a whole-program one).
function first_class_probe() {
    $rev = strrev(...);
    $sq = MathOps::square(...);
    return apply_twice($rev, 'ab') . ':' . $sq(4);
}
echo first_class_probe(), "\n";

// Exceptions thrown inside a closure body cross the dense call boundary
// into a catching (rich-planned) helper function.
function exception_probe() {
    $boom = function ($n) {
        if ($n > 1) {
            throw new RuntimeException("boom:$n");
        }
        return $n;
    };
    try {
        return apply_twice($boom, 1);
    } catch (RuntimeException $e) {
        return $e->getMessage();
    }
}
echo exception_probe(), "\n";

// (Calling an undefined function name through a callable string raises
// an uncatchable engine fatal instead of the reference's catchable
// Error - separate pre-existing known gap on both interpreters.)

// Nested higher-order dispatch keeps closure identity distinct.
$make_adder = function ($step) {
    return function ($n) use ($step) {
        return $n + $step;
    };
};
$add3 = $make_adder(3);
$add7 = $make_adder(7);
echo $add3(1), "|", $add7(1), "|", apply_twice($add3, 0), "\n";
