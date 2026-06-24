<?php
// expect=skip
function &slot() {
    static $x = 1;
    return $x;
}

$a =& slot();
$a = 7;
echo slot();
