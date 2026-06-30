<?php
class CallableBox {
    public function __invoke($value) {
        return $value . "I";
    }
}

function apply(callable $callback, $value) {
    echo $callback($value), "|";
}

function suffix($value) {
    return $value . "S";
}

apply("suffix", "A");
apply(function ($value) { return $value . "C"; }, "B");
apply(new CallableBox(), "C");
apply(strlen(...), "abcd");
