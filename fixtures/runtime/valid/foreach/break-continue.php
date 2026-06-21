<?php
foreach ([1, 2, 3, 4] as $value) {
    if ($value == 2) {
        continue;
    }
    if ($value == 4) {
        break;
    }
    echo $value;
}
echo "\n";
