<?php
function &pick_ref() {
    static $value = 1;
    return $value;
}

$x =& pick_ref();
echo $x;
