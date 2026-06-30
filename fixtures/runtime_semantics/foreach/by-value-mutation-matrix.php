<?php
// runtime-semantics: expect=pass
$items = [1, 2, 3, 4];
foreach ($items as $key => $value) {
    echo $key, ":", $value, ";";
    if ($key === 0) {
        unset($items[1]);
    }
    if ($key === 1) {
        $items[] = 5;
    }
    if ($key === 2) {
        $items[3] = 9;
    }
}
echo "|";
foreach ($items as $key => $value) {
    echo $key, ":", $value, ";";
}
echo "\n";
