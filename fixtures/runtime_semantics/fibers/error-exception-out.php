<?php
// runtime-semantics: expect=fail
$fiber = new Fiber(function (): void {
    Fiber::suspend("ready");
    throw new Exception("boom");
});

$fiber->start();
$fiber->resume(null);
