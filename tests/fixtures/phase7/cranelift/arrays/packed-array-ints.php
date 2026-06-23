<?php
function phase7_cranelift_packed_array_ints(): int {
    $xs = [3, 5, 7];
    return $xs[0] + $xs[1] + $xs[2];
}

echo phase7_cranelift_packed_array_ints(), "\n";
