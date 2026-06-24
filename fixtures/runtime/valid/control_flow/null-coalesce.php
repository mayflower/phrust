<?php
// runtime-fixture: kind=valid expected_stdout="fallback|value\n"
$value = "value";
echo $missing ?? "fallback";
echo "|";
echo $value ?? "bad";
echo "\n";
