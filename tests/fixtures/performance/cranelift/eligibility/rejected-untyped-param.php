<?php
function perf_cranelift_rejected_untyped($value): int {
    return $value + 1;
}

echo perf_cranelift_rejected_untyped(4), "\n";
