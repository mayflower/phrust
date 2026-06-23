<?php
// phase6-diff: id=PHASE6_STDLIB_EXCEPTION_HANDLER area=stdlib expect=pass
function phase6_exception_handler($e) {
    echo "handled:", $e->getMessage(), "\n";
}
echo set_exception_handler('phase6_exception_handler') === null ? "first-null\n" : "bad\n";
throw new Exception('boom');
echo "after\n";
