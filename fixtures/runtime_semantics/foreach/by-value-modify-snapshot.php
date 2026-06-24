<?php
$items = [1, 2];
foreach ($items as $value) {
    echo $value;
    $items[0] = 9;
}
echo "|", $items[0], "\n";
