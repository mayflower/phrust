<?php
// stdlib-diff: id=STDLIB_ARRAY_NATURAL_SORT_EDGE area=stdlib expect=known_gap known_gap=STDLIB-GAP-NATURAL-SORT-EDGE-CASES
$values = ["a02", "a2", "a002", "a10"];
natsort($values);
echo var_export($values, true), "\n";
