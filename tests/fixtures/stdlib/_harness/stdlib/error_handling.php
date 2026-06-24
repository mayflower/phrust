<?php
// stdlib-diff: id=STDLIB_ERROR_HANDLING area=stdlib expect=pass
function stdlib_first_error($errno, $errstr, $errfile, $errline) {
    echo "first:", $errno, ":", $errstr, "\n";
    return true;
}

function stdlib_second_error($errno, $errstr, $errfile, $errline) {
    echo "second:", $errno, ":", $errstr, "\n";
    return true;
}

echo set_error_handler('stdlib_first_error') === null ? "first-null\n" : "bad\n";
set_error_handler('stdlib_second_error');
trigger_error('top', E_USER_WARNING);
restore_error_handler();
user_error('restored', E_USER_WARNING);
restore_error_handler();
