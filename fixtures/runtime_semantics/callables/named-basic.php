<?php
// runtime-semantics: category=callables expect=pass
function join_values($first, $second = "B", ...$rest) {
    echo $first, "|", $second, "|", $rest["third"];
}

join_values(second: "S", first: "F", third: "R1");
