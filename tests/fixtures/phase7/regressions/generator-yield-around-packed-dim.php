<?php
function phase744_numbers() {
    $items = [1, 2, 3];
    for ($i = 0; $i < 12; $i++) {
        yield $items[1] + $i;
    }
}

foreach (phase744_numbers() as $value) {
    echo $value, '|';
}
echo "\n";
