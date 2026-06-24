<?php
// runtime-semantics: category=callables expect=pass
function inc($value) {
    return $value + 1;
}

$callable = "inc";
echo $callable(4);
