<?php
$items = [1, 2, 3];
foreach ($items as &$value) {
    echo $value, ";";
    if ($value === 2) {
        break;
    }
}
$value = 9;
echo "|", $items[1], "|";
foreach ($items as $key => $seen) {
    echo $key, ":", $seen, ";";
}
echo "\n";
