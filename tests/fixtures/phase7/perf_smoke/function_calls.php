<?php
function phase7_add($left, $right) {
    return $left + $right;
}

$value = 0;
for ($i = 0; $i < 5; $i++) {
    $value = phase7_add($value, $i);
}
echo "calls:", $value, "\n";
