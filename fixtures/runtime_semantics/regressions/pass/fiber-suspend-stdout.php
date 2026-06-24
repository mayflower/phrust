<?php
// runtime-semantics: expect=pass regression_category=fibers reference_behavior=stdout:as|b regression_case=48
$fiber = new Fiber(function () {
    echo "a";
    Fiber::suspend("s");
    echo "b";
});
echo $fiber->start(), "|";
$fiber->resume("r");
