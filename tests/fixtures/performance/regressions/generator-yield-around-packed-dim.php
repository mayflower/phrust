<?php
function performance44_numbers() {
    $items = [1, 2, 3];
    for ($i = 0; $i < 12; $i++) {
        yield $items[1] + $i;
    }
}

foreach (performance44_numbers() as $value) {
    echo $value, '|';
}
echo "\n";
