<?php
// runtime-fixture: kind=valid expected_stdout="012\n"
$i = 0;
while ($i < 3) {
    echo $i;
    $i++;
}
echo "\n";
