<?php
// phase5-runtime: expect=fail
$fiber = new Fiber(function (): void {
    Fiber::suspend("ready");
});

$fiber->start();
$fiber->getReturn();
