<?php
// phase6-diff: id=PHASE6_STDLIB_MATH_NUMERIC_FLOAT_EDGES area=stdlib expect=known_gap known_gap=PHASE6-GAP-MATH-FLOAT-EDGES
echo var_export(round(2.5, 0, PHP_ROUND_HALF_DOWN), true), "\n";
echo number_format(-0.01, 0), "\n";
echo var_export(pow(9223372036854775807, 2), true), "\n";
