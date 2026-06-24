<?php
// runtime-semantics: expect=fail
function inc_ref(&$x) {
    $x = $x + 1;
}

$a = 1;
inc_ref($a + 1);
