<?php
function set_ref(&$value) {
    $value = 2;
}

$x = 1;
set_ref($x);
echo $x;
