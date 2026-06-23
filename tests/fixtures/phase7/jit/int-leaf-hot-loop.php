<?php
function phase7_jit_add($a, $b) {
    return $a + $b;
}

$sum = 0;
for ($i = 0; $i < 12; $i++) {
    $sum = $sum + phase7_jit_add($i, 2);
}
echo $sum, "\n";
