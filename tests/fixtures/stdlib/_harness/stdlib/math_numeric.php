<?php
// stdlib-diff: id=STDLIB_MATH_NUMERIC area=stdlib expect=pass
echo var_export(abs(-7), true), "\n";
echo var_export(abs("-2.5"), true), "\n";
echo var_export(min([3, 1, 2]), true), "\n";
echo var_export(max(3, 1, 2), true), "\n";
echo var_export(round(12.345, 2), true), "\n";
echo var_export(floor(3.9), true), "\n";
echo var_export(ceil(3.1), true), "\n";
echo var_export(sqrt(9), true), "\n";
echo var_export(pow(2, 3), true), "\n";
echo var_export(intdiv(7, 2), true), "\n";
echo var_export(fmod(7, 2), true), "\n";
echo is_finite("1.5") ? "T\n" : "F\n";
echo is_infinite(1e309) ? "T\n" : "F\n";
echo is_nan(sqrt(-1)) ? "T\n" : "F\n";
echo number_format(1234.567, 2), "\n";
echo number_format(1234.5, 1, ",", "."), "\n";
