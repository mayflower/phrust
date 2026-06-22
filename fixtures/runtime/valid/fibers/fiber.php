<?php
// phase5: kind=pass id=fiber-start
$fiber = new Fiber(function () {
    echo "fiber";
});

$fiber->start();
