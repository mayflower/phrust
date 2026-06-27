<?php
$items = [];
for ($i = 0; $i < 12; $i++) {
    $items[] = $i + 1;
}
$sum = 0;
for ($i = 0; $i < 12; $i++) {
    $sum += $items[1];
}
$foreachSum = 0;
foreach ($items as $value) {
    $foreachSum += $value;
}
echo "packed:", count($items), ":", $sum, ":", $foreachSum, "\n";
