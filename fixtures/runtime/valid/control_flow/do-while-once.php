<?php
// runtime-fixture: kind=valid expected_stdout="once\n"
$i = 0;
do {
    echo "once\n";
    $i++;
} while ($i < 1);
