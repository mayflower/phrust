<?php
// phase4: kind=valid expected_stdout="12\n"
$i = 0;
while ($i < 5) {
    $i++;
    if ($i == 3) {
        break;
    }
    echo $i;
}
echo "\n";
