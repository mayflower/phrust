<?php
$items = [1, 2, 3, 4];
$assoc = ["a" => 1, "b" => 2, "c" => 3];
$sum = 0;
for ($i = 0; $i < 8; $i++) {
    $sum += count($items);
    $sum += strlen("abcd");
    $sum += is_int($i) ? 1 : 0;
    $sum += count(array_values($assoc));
    $sum += strlen(strtolower("ABC"));
}
echo "stdlib:", $sum, "\n";
