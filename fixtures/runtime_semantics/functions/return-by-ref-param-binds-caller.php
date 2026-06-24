<?php
function &identity_ref(&$x) {
    return $x;
}

$a = 1;
$b =& identity_ref($a);
$b = 4;
echo $a, "|", $b;
