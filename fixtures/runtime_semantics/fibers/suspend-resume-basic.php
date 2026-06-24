<?php
$fiber = new Fiber(function (): string {
    echo "before\n";
    $value = Fiber::suspend("pause");
    echo "after:", $value, "\n";
    return "done";
});

var_dump($fiber->start());
var_dump($fiber->isSuspended());
var_dump($fiber->resume("go"));
var_dump($fiber->isTerminated());
var_dump($fiber->getReturn());
