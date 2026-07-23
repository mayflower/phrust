<?php
// runtime-semantics: category=callables expect=pass php_ref_required=1

class ClosureBindFixtureSecret
{
    private int $value = 10;
}

$closure = function () {
    return $this->value;
};
$target = new ClosureBindFixtureSecret();
$bound = Closure::bind($closure, $target, ClosureBindFixtureSecret::class);
$boundTo = $closure->bindTo($target, ClosureBindFixtureSecret::class);

echo $bound(), "\n";
echo $boundTo(), "\n";
echo spl_object_id($closure) === spl_object_id($bound) ? "same\n" : "distinct\n";
