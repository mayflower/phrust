<?php
// runtime-semantics: category=known_gaps expect=known_gap known_gap=E_PHP_RUNTIME_UNSUPPORTED_CLOSURE_BINDING
// PHP reference: Closure::bind can bind both $this and the class scope.
class ClosureBindFixtureSecret
{
    private string $value = "bound";
}

$closure = function () {
    return $this->value;
};
$bound = Closure::bind($closure, new ClosureBindFixtureSecret(), ClosureBindFixtureSecret::class);
echo $bound(), "\n";
