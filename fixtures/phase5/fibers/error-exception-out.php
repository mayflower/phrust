<?php
// phase5-runtime: expect=fail
$fiber = new Fiber(function (): void {
    Fiber::suspend("ready");
    throw new Exception("boom");
});

$fiber->start();
$fiber->resume(null);
