<?php
// phase5-runtime: category=generators expect=pass
function inner() {
    yield "x" => 3;
    return 9;
}

function outer() {
    $result = yield from inner();
    echo "return:", $result, "\n";
}

foreach (outer() as $key => $value) {
    echo $key, ":", $value, "\n";
}
