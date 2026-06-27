<?php
$values = [1, "two", [3]];
$sum = 0;
for ($i = 0; $i < 8; $i++) {
    $sum += strlen("abcd");
    $sum += count($values);
    $sum += is_int($i) ? 1 : 0;
    $sum += is_string("x") ? 1 : 0;
    $sum += is_array($values) ? 1 : 0;
    $sum += strlen($i);
    $sum += is_array("x") ? 1 : 0;
}
echo "builtin-stubs:", $sum, "\n";
