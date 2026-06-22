<?php
$fiber = new Fiber(function (): void {});
var_dump($fiber instanceof Fiber);
var_dump($fiber->isStarted());
var_dump($fiber->isSuspended());
var_dump($fiber->isRunning());
var_dump($fiber->isTerminated());
