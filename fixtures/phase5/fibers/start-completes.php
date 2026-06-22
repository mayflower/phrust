<?php
$fiber = new Fiber(function ($name): string {
    echo "fiber:", $name, "\n";
    return strtoupper($name);
});

var_dump($fiber->start("ok"));
var_dump($fiber->isStarted());
var_dump($fiber->isSuspended());
var_dump($fiber->isRunning());
var_dump($fiber->isTerminated());
