<?php
// runtime-semantics: expect=fail
$fiber = new Fiber(function (): void {
    Fiber::suspend("ready");
});

$fiber->start();
$fiber->getReturn();
