<?php
$items = [1, 2, 3];
foreach ($items as $value) {
    echo $value;
    unset($items[1]);
}
echo "|";
foreach ($items as $value) {
    echo $value;
}
echo "\n";
