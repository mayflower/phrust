<?php
// runtime-fixture: kind=pass id=fiber-start
$fiber = new Fiber(function () {
    echo "fiber";
});

$fiber->start();
