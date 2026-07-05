--TEST--
calendar info, day names, Easter days, and Unix JD helpers
--SKIPIF--
<?php if (!extension_loaded("calendar")) die("skip calendar extension not loaded"); ?>
--FILE--
<?php
$jd = gregoriantojd(7, 4, 2026);
var_dump(CAL_GREGORIAN, CAL_JULIAN, CAL_NUM_CALS);
var_dump(jddayofweek($jd), jddayofweek($jd, CAL_DOW_SHORT), jddayofweek($jd, CAL_DOW_LONG));
var_dump(jdmonthname($jd, CAL_MONTH_GREGORIAN_SHORT), jdmonthname($jd, CAL_MONTH_GREGORIAN_LONG));
var_dump(jdtounix(2440588), unixtojd(0));
var_dump(easter_days(2026), easter_date(2026));
$info = cal_info(CAL_GREGORIAN);
echo $info["calname"], "|", $info["calsymbol"], "|", $info["months"][7], "\n";
?>
--EXPECT--
int(0)
int(1)
int(4)
int(6)
string(3) "Sat"
string(8) "Saturday"
string(3) "Jul"
string(4) "July"
int(0)
int(2440588)
int(15)
int(1775347200)
Gregorian|CAL_GREGORIAN|July
