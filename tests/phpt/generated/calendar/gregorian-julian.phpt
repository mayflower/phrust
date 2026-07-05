--TEST--
calendar Gregorian and Julian conversion slice
--SKIPIF--
<?php if (!extension_loaded("calendar")) die("skip calendar extension not loaded"); ?>
--FILE--
<?php
$jd = gregoriantojd(7, 4, 2026);
var_dump($jd);
var_dump(jdtogregorian($jd));
var_dump(juliantojd(6, 21, 2026));
var_dump(jdtojulian($jd));
var_dump(cal_to_jd(CAL_GREGORIAN, 2, 29, 2024));
var_dump(cal_days_in_month(CAL_GREGORIAN, 2, 2024));
$from = cal_from_jd($jd, CAL_GREGORIAN);
echo $from["date"], "|", $from["dayname"], "|", $from["monthname"], "\n";
?>
--EXPECT--
int(2461226)
string(8) "7/4/2026"
int(2461226)
string(9) "6/21/2026"
int(2460370)
int(29)
7/4/2026|Saturday|July
