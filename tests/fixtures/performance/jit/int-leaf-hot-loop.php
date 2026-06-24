<?php
function perf_jit_add(int $a, int $b): int {
    return $a + $b;
}

$sum = 0;
for ($i = 0; $i < 12; $i++) {
    $sum = $sum + perf_jit_add($i, 2);
}
echo $sum, "\n";
