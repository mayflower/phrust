--TEST--
Generated zend.functions: a throwing exception handler routes to the current handler
--DESCRIPTION--
module: zend.functions
generated timestamp: 20260627T000000Z
generator version: phpt-zend-functions-v1
reason: an exception thrown by a set_exception_handler() callback is dispatched to the handler active at that point, not leaked as an internal error (Zend/tests/exceptions/gh10695_5.phpt)
--FILE--
<?php
set_exception_handler(function (\Throwable $exception) {
    echo 'Caught: ' . $exception->getMessage() . "\n";
    set_exception_handler(function (\Throwable $exception) {
        echo 'Caught: ' . $exception->getMessage() . "\n";
    });
    throw new \Exception('exception handler');
});

throw new \Exception('main');
?>
--EXPECT--
Caught: main
Caught: exception handler
