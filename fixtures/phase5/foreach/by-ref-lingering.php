<?php
$items = [1, 2];
foreach ($items as &$value) {
}
$value = 9;
echo $items[1], "\n";
