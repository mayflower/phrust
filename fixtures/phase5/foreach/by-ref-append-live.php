<?php
$items = [1, 2];
$done = false;
foreach ($items as &$value) {
    echo $value;
    if (!$done) {
        $items[] = 3;
        $done = true;
    }
}
unset($value);
echo "\n";
