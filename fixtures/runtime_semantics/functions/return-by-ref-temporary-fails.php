<?php
// runtime-semantics: expect=fail
function &bad_ref() {
    return 1;
}

$x =& bad_ref();
echo $x;
