<?php
$values = [1, "two", [3]];
$shape = ["answer" => 42, 3 => "three"];
$sum = 0;
for ($i = 0; $i < 8; $i++) {
    $sum += strlen("abcd");
    $sum += count($values);
    $sum += is_int($i) ? 1 : 0;
    $sum += is_string("x") ? 1 : 0;
    $sum += is_array($values) ? 1 : 0;
    $sum += array_key_exists("answer", $shape) ? 1 : 0;
    $sum += array_key_exists($i, $shape) ? 1 : 0;
    $sum += strlen($i);
    $sum += is_array("x") ? 1 : 0;
}
try {
    array_key_exists([], $shape);
} catch (TypeError $e) {
    $sum += 0;
}
echo "builtin-stubs:", $sum, "\n";
