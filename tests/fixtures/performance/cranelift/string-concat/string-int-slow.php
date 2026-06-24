<?php
function perf_cranelift_concat_string_int(string $lhs, int $rhs): string {
    return $lhs . $rhs;
}

echo perf_cranelift_concat_string_int("id-", 42), "\n";
