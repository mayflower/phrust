<?php
function phase744_strings() {
    $value = '';
    for ($i = 0; $i < 12; $i++) {
        $value = $value . 'x';
        yield $value;
    }
}

foreach (phase744_strings() as $value) {
    echo $value, '|';
}
echo "\n";
