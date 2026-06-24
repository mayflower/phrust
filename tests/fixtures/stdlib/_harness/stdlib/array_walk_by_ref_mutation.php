<?php
// stdlib-diff: id=STDLIB_ARRAY_WALK_BY_REF_MUTATION area=stdlib expect=known_gap known_gap=STDLIB-GAP-ARRAY-WALK-BY-REF-MUTATION
$values = [1, 2];
array_walk($values, function(&$value) { $value = $value * 10; });
echo var_export($values, true), "\n";
