<?php
$items = [1];
$alias =& $items[0];
foreach ($items as $value) {
    $value = 9;
    echo $items[0], ":", $alias, ":", $value;
}
echo "\n";
