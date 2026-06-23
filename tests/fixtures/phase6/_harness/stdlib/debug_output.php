<?php
// phase6-diff: id=PHASE6_STDLIB_DEBUG_OUTPUT area=stdlib expect=pass
var_dump(null, true, 7, "hi", [1, "x"]);
echo "---\n";
print_r([1, "x"]);
echo "---\n";
echo print_r([1], true);
echo "---\n";
var_export([1, "x"]);
echo "\n---\n";
echo var_export([1], true), "\n";
