<?php
$count = 0;
for ($outer = 0; $outer < 4; $outer++) {
    $inner = 0;
    while ($inner < 3) {
        $count += $outer + $inner;
        $inner++;
    }
}
echo "loops:", $count, "\n";
