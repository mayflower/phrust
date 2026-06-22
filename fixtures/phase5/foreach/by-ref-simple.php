<?php
$items = [1, 2];
foreach ($items as &$value) {
    $value = $value + 10;
}
unset($value);
foreach ($items as $value) {
    echo $value;
}
echo "\n";
