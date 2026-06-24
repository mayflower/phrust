<?php
function perf_cranelift_packed_foreach_sum_large(array $xs): int {
    $sum = 0;
    foreach ($xs as $item) {
        $sum += $item;
    }
    return $sum;
}

$xs = [];
for ($i = 0; $i < 256; $i++) {
    $xs[] = $i;
}

$result = 0;
for ($i = 0; $i < 64; $i++) {
    $result += perf_cranelift_packed_foreach_sum_large($xs);
}

echo $result, "\n";
