<?php
function perf_cranelift_mixed_array_fallback(): int {
    $xs = [0 => 3, "name" => 5, 2 => 7];
    return $xs[0] + $xs["name"] + $xs[2];
}

echo perf_cranelift_mixed_array_fallback(), "\n";
