<?php
// runtime-semantics: category=callables expect=pass
class Greeter {
    public function join($left, $right) {
        return $left . $right;
    }
}

$callable = [new Greeter(), "join"];
echo $callable("A", "B");
