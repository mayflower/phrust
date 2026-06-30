<?php
function by_ref_default(&$x = null) {
    var_dump($x);
    $x = 7;
    var_dump($x);
}

by_ref_default();
