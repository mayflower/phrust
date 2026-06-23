<?php
function phase7_cranelift_count_packed(array $value): int {
    return count($value);
}

echo phase7_cranelift_count_packed([10, 20, 30]), "\n";
