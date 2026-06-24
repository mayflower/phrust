<?php
// runtime-fixture: kind=valid expected_stdout="134\n"
$i = 0;
while ($i < 4) {
    $i++;
    if ($i == 2) {
        continue;
    }
    echo $i;
}
echo "\n";
