<?php
function rejected_dynamic($value) {
    $items = [];
    $items[] = strlen($value);
    return $items[0];
}

echo rejected_dynamic('abc');
