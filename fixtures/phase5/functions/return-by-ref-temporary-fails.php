<?php
// phase5-runtime: expect=fail
function &bad_ref() {
    return 1;
}

$x =& bad_ref();
echo $x;
