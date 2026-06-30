<?php
function bump_global_ref(&$value) {
    $value++;
}

function run_global_ref() {
    global $value;
    bump_global_ref($value);
}

$value = 1;
run_global_ref();
echo $value;
