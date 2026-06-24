<?php
function perf_jit_reject($value): int {
    $items = [];
    $items[] = strlen($value);
    return $items[0];
}

$sum = 0;
for ($i = 0; $i < 8; $i++) {
    $sum = $sum + perf_jit_reject("abcd");
}
echo $sum, "\n";
