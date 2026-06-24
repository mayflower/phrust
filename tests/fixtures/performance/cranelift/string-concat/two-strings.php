<?php
function perf_cranelift_concat_two_strings(string $lhs, string $rhs): string {
    return $lhs . $rhs;
}

echo perf_cranelift_concat_two_strings("hello", "-world"), "\n";
