<?php
// stdlib-diff: id=STDLIB_OUTPUT_BUFFERING_EXCEPTION area=stdlib expect=pass
ob_start();
try {
    echo "before|";
    throw new Exception('boom');
} catch (Exception $e) {
    echo "catch|";
}
echo ob_get_clean(), "\n";
ob_start();
echo "shutdown";
