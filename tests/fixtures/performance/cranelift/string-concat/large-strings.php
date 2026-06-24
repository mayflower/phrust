<?php
function perf_cranelift_concat_large_strings(string $lhs, string $rhs): string {
    return $lhs . $rhs;
}

$left = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
$right = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
echo strlen(perf_cranelift_concat_large_strings($left, $right)), "\n";
