<?php
// phase6-diff: id=PHASE6_STDLIB_ARRAY_STACK_SPLICE area=stdlib expect=pass
$a = [1, 2];
echo array_push($a, 3), "|", var_export($a, true), "\n";
echo var_export(array_pop($a), true), "|", var_export($a, true), "\n";
echo array_unshift($a, 0), "|", var_export($a, true), "\n";
echo var_export(array_shift($a), true), "|", var_export($a, true), "\n";
$b = ["a", "b", "c"];
echo var_export(array_splice($b, 1, 1, ["x"]), true), "|", var_export($b, true), "\n";
