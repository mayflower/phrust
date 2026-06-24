<?php
$fiber = new Fiber(function (): string {
    try {
        Fiber::suspend("ready");
    } catch (Exception $e) {
        echo $e->getMessage(), "\n";
        return "caught";
    }
    return "miss";
});

var_dump($fiber->start());
var_dump($fiber->throw(new Exception("boom")));
var_dump($fiber->getReturn());
