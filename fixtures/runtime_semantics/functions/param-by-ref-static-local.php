<?php
function bump_static_local_ref(&$value) {
    $value++;
}

function run_static_local_ref() {
    static $value = 1;
    bump_static_local_ref($value);
    echo $value;
}

run_static_local_ref();
