<?php
// phase4: kind=known_gap id=E_PHP_IR_UNSUPPORTED_FIBER
$fiber = new Fiber(function () {
    echo "fiber";
});

$fiber->start();
