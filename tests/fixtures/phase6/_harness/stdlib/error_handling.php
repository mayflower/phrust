<?php
// phase6-diff: id=PHASE6_STDLIB_ERROR_HANDLING area=stdlib expect=pass
function phase6_first_error($errno, $errstr, $errfile, $errline) {
    echo "first:", $errno, ":", $errstr, "\n";
    return true;
}

function phase6_second_error($errno, $errstr, $errfile, $errline) {
    echo "second:", $errno, ":", $errstr, "\n";
    return true;
}

echo set_error_handler('phase6_first_error') === null ? "first-null\n" : "bad\n";
set_error_handler('phase6_second_error');
trigger_error('top', E_USER_WARNING);
restore_error_handler();
user_error('restored', E_USER_WARNING);
restore_error_handler();
