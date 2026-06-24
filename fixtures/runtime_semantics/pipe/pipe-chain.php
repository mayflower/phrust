<?php
// runtime-semantics: category=pipe expect=pass
function inc($value) {
    return $value + 1;
}

function double($value) {
    return $value * 2;
}

echo 3 |> inc(...) |> double(...);
