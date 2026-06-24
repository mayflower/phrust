<?php
// runtime-semantics: expect=fail
$fiber = new Fiber(function (): void {});
$fiber->start();
$fiber->start();
