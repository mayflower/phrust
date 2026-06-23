<?php
function phase7_cranelift_rejected_array(int $value): int {
    $items = [$value];
    return $items[0];
}

echo phase7_cranelift_rejected_array(7), "\n";
