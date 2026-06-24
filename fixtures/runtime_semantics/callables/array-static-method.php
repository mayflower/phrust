<?php
// runtime-semantics: category=callables expect=pass
class Tools {
    public static function wrap($value) {
        return $value . "!";
    }
}

$callable = ["Tools", "wrap"];
echo $callable("x");
