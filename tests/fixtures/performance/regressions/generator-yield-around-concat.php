<?php
function performance44_strings() {
    $value = '';
    for ($i = 0; $i < 12; $i++) {
        $value = $value . 'x';
        yield $value;
    }
}

foreach (performance44_strings() as $value) {
    echo $value, '|';
}
echo "\n";
