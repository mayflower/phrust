<?php
function phase7_cranelift_count_mixed(array $value): int {
    return count($value);
}

echo phase7_cranelift_count_mixed(["a" => 10, 4 => 20, "b" => 30]), "\n";
