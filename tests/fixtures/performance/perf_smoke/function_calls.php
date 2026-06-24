<?php
function perf_add($left, $right) {
    return $left + $right;
}

$value = 0;
for ($i = 0; $i < 5; $i++) {
    $value = perf_add($value, $i);
}
echo "calls:", $value, "\n";
