<?php
function phase7_cranelift_concat_string_int(string $lhs, int $rhs): string {
    return $lhs . $rhs;
}

echo phase7_cranelift_concat_string_int("id-", 42), "\n";
