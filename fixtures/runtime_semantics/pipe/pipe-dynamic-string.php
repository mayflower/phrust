<?php
// runtime-semantics: category=pipe expect=pass
function inc($value) {
    return $value + 1;
}

$callable = "inc";
echo 4 |> $callable;
