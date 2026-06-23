<?php
// phase6-diff: id=PHASE6_STDLIB_ARRAY_FLIP_WARNING area=stdlib
echo var_export(array_flip(["ok" => "value", "bad" => []]), true), "\n";
