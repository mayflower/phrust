<?php
// phase6-diff: id=PHASE6_STDLIB_ERROR_REPORTING area=stdlib expect=pass
function phase6_reporting_handler($errno, $errstr, $errfile, $errline) {
    echo "handled:", $errstr, "\n";
    return true;
}
error_reporting(0);
echo error_reporting(), "\n";
trigger_error('masked', E_USER_WARNING);
error_reporting(E_USER_WARNING);
set_error_handler('phase6_reporting_handler', E_USER_WARNING);
trigger_error('visible', E_USER_WARNING);
echo error_reporting(), "\n";
