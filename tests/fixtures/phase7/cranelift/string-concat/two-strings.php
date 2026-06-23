<?php
function phase7_cranelift_concat_two_strings(string $lhs, string $rhs): string {
    return $lhs . $rhs;
}

echo phase7_cranelift_concat_two_strings("hello", "-world"), "\n";
