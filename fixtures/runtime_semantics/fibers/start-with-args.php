<?php
$fiber = new Fiber(function ($a, $b): void {
    echo $a + $b, "\n";
});

$fiber->start(2, 5);
var_dump($fiber->isTerminated());
