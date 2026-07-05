--TEST--
calendar Jewish and French republican conversion functions
--SKIPIF--
<?php if (!extension_loaded("calendar")) die("skip calendar extension not loaded"); ?>
--FILE--
<?php
var_dump(jewishtojd(-1, -1, -1));
var_dump(jewishtojd(1, 1, 1));
var_dump(jewishtojd(2, 22, 5763));
var_dump(jdtojewish(2452576));
var_dump(frenchtojd(-1, -1, -1));
var_dump(frenchtojd(1, 1, 1));
var_dump(frenchtojd(14, 31, 15));
var_dump(jdtofrench(0));
var_dump(jdtofrench(2375840));
var_dump(jdtofrench(2375940));
var_dump(cal_to_jd(CAL_JEWISH, 2, 22, 5763));
var_dump(cal_to_jd(CAL_FRENCH, 1, 1, 1));
$jewish = cal_from_jd(2453396, CAL_JEWISH);
echo $jewish["date"], "|", $jewish["monthname"], "\n";
$french = cal_from_jd(2375840, CAL_FRENCH);
echo $french["date"], "|", $french["monthname"], "\n";
var_dump(jdmonthname(2453396, CAL_MONTH_JEWISH));
var_dump(jdmonthname(2375840, CAL_MONTH_FRENCH));
?>
--EXPECT--
int(0)
int(347998)
int(2452576)
string(9) "2/22/5763"
int(0)
int(2375840)
int(0)
string(5) "0/0/0"
string(5) "1/1/1"
string(6) "4/11/1"
int(2452576)
int(2375840)
5/15/5765|Shevat
1/1/1|Vendemiaire
string(6) "Shevat"
string(11) "Vendemiaire"
