<?php
function bump_array_ref(&$value) {
    $value++;
}

$items = [1];
bump_array_ref($items[0]);
echo $items[0];
