<?php
$items = [1, 2, 3];
$ref =& $items[1];

for ($i = 0; $i < 12; $i++) {
    $items[1] = $items[1] + 1;
    if ($i === 5) {
        $ref = 20;
    }
    echo $items[1], ':', $ref, '|';
}
echo "\n";
