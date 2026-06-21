<?php
$items = [1, 2];
foreach ($items as $value) {
    echo $value;
    $items[] = 9;
}
echo "|";
foreach ($items as $value) {
    echo $value;
}
echo "\n";
