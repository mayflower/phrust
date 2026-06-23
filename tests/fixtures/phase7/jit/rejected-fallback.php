<?php
function phase7_jit_reject($value): int {
    return strlen($value);
}

$sum = 0;
for ($i = 0; $i < 8; $i++) {
    $sum = $sum + phase7_jit_reject("abcd");
}
echo $sum, "\n";
