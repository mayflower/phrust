<?php
// runtime-semantics: expect=fail
function value_only() {
    $x = 1;
    return $x;
}

$r =& value_only();
echo $r;
