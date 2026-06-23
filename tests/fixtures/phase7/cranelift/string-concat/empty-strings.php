<?php
function phase7_cranelift_concat_empty_strings(string $lhs, string $rhs): string {
    return $lhs . $rhs;
}

echo "[", phase7_cranelift_concat_empty_strings("", ""), "]\n";
