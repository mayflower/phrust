<?php
// phase6-diff: id=PHASE6_STDLIB_STRING_TRANSFORM area=stdlib expect=pass
echo var_export(explode(",", "a,b,c"), true), "\n";
echo var_export(explode(",", "a,b,c", -1), true), "\n";
echo implode("|", ["a", "b"]), "\n";
$count = 0;
echo str_replace(["a", "b"], ["x", "y"], "abca", $count), "|", $count, "\n";
echo var_export(str_replace("a", "x", ["a", "ba"]), true), "\n";
echo strtr("abc", "ab", "xy"), "|", trim(" x "), "|", ltrim(" x "), "|", rtrim(" x "), "\n";
echo strtolower("AbC"), "|", strtoupper("AbC"), "|", ucfirst("abc"), "|", lcfirst("Abc"), "|", ucwords("a b"), "\n";
echo str_repeat("ab", 3), "|", str_pad("x", 3, "0", 0), "|", str_pad("x", 4, "0", 2), "|", strrev("abc"), "\n";
