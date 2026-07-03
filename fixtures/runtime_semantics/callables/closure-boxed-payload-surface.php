<?php
// runtime-semantics: category=callables expect=pass php_ref_required=1
// Exercises the closure payload surface end to end: captures, bindTo,
// first-class callable syntax, __invoke dispatch, callable type checks,
// and var_dump of a closure value.
class BoxSurface
{
    private $seed;

    public function __construct($seed)
    {
        $this->seed = $seed;
    }

    public function scale($factor)
    {
        return $this->seed * $factor;
    }
}

$captured = 5;
$closure = function ($x) use ($captured) {
    return $x + $captured;
};
echo $closure(2), "\n";

$method = new BoxSurface(3)->scale(...);
echo $method(4), "\n";

$strlen = strlen(...);
echo $strlen("boxed"), "\n";

$bound = Closure::bind(function () {
    return $this->seed * 10;
}, new BoxSurface(7), BoxSurface::class);
echo $bound(), "\n";

echo is_callable($closure) ? "callable" : "not-callable", "\n";
echo gettype($closure), "\n";
echo $closure instanceof Closure ? "closure" : "other", "\n";
var_dump($closure);
