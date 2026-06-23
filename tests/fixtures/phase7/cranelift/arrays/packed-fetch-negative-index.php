<?php
function phase7_cranelift_packed_fetch_negative_index(array $xs, int $i): int {
    return $xs[$i];
}

echo phase7_cranelift_packed_fetch_negative_index([10], -1), "\n";
