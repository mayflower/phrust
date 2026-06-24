<?php
// runtime-fixture: kind=valid expected_stdout="zeroone\n"
$x = 0;
switch ($x) {
    case 0:
        echo "zero";
    case 1:
        echo "one";
        break;
    default:
        echo "default";
}
echo "\n";
