<?php
// runtime-semantics: category=wordpress_blockers expect=pass
function pick($value, $left, $right)
{
    return $value ? $left : $right;
}

function mark($label)
{
    echo $label, "|";
    return $label;
}

echo (true ? "yes" : "no"), "\n";
echo (false ? "yes" : "no"), "\n";
$x = 0;
echo ($x ? "bad" : "ok"), "\n";
echo ((1 < 2) ? "lt" : "ge"), "\n";
echo (false ?: "fallback"), "\n";
$maybe = null;
echo ($maybe ?? "coalesce"), "\n";
$assigned = true ? "assign" : "bad";
echo $assigned, "\n";
echo pick(true, "arg-yes", "arg-no"), "\n";
echo (pick(false, "concat-bad", "concat-ok") . "\n");
echo (true ? mark("selected") : mark("bad")), "\n";
