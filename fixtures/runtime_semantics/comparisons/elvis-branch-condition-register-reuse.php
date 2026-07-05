<?php
// Regression: `local ?: fallback` branches on the loaded local while the
// truthy arm reuses the same register as the expression result. A branch
// fusion that skips the register write on locals used "only" as branch
// predicates breaks every truthy elvis/ternary over a local.
$values = ["falsy" => 0, "truthy" => true, "string" => "x", "null" => null];
foreach ($values as $label => $value) {
    $elvis = $value ?: "E";
    $ternary = $value ? $value : "T";
    echo $label, ":", $elvis === "E" ? "E" : "V";
    echo $ternary === "T" ? "T" : "V", "\n";
}
$single = "kept";
echo $single ?: "lost", "\n";
$zero = 0;
echo $zero ?: "fallback", "\n";
