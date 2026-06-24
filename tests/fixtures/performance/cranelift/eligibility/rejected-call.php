<?php
function perf_cranelift_rejected_call(string $value): int {
    return strlen($value);
}

echo perf_cranelift_rejected_call("abcd"), "\n";
