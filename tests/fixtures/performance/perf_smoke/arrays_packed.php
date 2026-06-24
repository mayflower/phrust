<?php
$items = [];
for ($i = 0; $i < 12; $i++) {
    $items[] = $i + 1;
}
$sum = 0;
for ($i = 0; $i < 12; $i++) {
    $sum += $items[1];
}
echo "packed:", count($items), ":", $sum, "\n";
