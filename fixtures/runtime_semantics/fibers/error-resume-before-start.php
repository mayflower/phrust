<?php
// runtime-semantics: expect=fail
$fiber = new Fiber(function (): void {});
$fiber->resume(null);
