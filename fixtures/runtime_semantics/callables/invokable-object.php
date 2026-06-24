<?php
// runtime-semantics: category=callables expect=pass
class Doubler {
    public function __invoke($value) {
        return $value * 2;
    }
}

$callable = new Doubler();
echo $callable(6);
