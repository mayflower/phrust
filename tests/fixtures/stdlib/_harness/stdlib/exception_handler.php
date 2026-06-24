<?php
// stdlib-diff: id=STDLIB_EXCEPTION_HANDLER area=stdlib expect=pass
function stdlib_exception_handler($e) {
    echo "handled:", $e->getMessage(), "\n";
}
echo set_exception_handler('stdlib_exception_handler') === null ? "first-null\n" : "bad\n";
throw new Exception('boom');
echo "after\n";
