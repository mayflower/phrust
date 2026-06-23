<?php
function phase7_cranelift_packed_fetch_mixed_array(array $xs, int $i): int {
    return $xs[$i];
}

$xs = [0 => 11, "name" => 12];
echo phase7_cranelift_packed_fetch_mixed_array($xs, 0), "\n";
