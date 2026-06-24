<?php
// stdlib-diff: id=STDLIB_VARIABLE_CASTS area=stdlib expect=pass
echo boolval(null) ? "1" : "0";
echo boolval("") ? "1" : "0";
echo boolval("0") ? "1" : "0";
echo boolval("x") ? "1" : "0";
echo "|";
echo intval(null), "|", intval(true), "|", intval("12abc"), "|", intval([1]), "\n";
echo floatval(null), "|", floatval(true), "|", floatval("1.5x"), "|", floatval([1]), "\n";
echo strval(null), "|", strval(false), "|", strval(true), "|", strval(12), "|", strval("x"), "\n";
