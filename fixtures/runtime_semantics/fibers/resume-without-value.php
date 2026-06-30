<?php
$fiber = new Fiber(function () {
    $value = Fiber::suspend("first");
    var_dump($value);
    return "done";
});

var_dump($fiber->start());
var_dump($fiber->resume());
var_dump($fiber->getReturn());
