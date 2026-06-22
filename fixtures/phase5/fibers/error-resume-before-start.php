<?php
// phase5-runtime: expect=fail
$fiber = new Fiber(function (): void {});
$fiber->resume(null);
