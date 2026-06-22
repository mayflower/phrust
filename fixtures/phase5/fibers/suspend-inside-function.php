<?php
function suspend_in_helper(): string
{
    $value = Fiber::suspend("helper");
    return "resumed:" . $value;
}

$fiber = new Fiber(function (): string {
    echo "enter\n";
    $value = suspend_in_helper();
    echo $value, "\n";
    return "done";
});

var_dump($fiber->start());
var_dump($fiber->resume("ok"));
var_dump($fiber->getReturn());
