--TEST--
Generated standard.strings: printf renders non-finite floats as INF/-INF/NaN
--DESCRIPTION--
module: standard.strings
generated timestamp: 20260626T000000Z
generator version: phpt-standard-strings-v1
reason: %f/%e/%g of INF, -INF and NAN print bare INF/-INF/NaN and ignore width, zero-fill, precision and the + flag (tests/strings/002.phpt)
--FILE--
<?php
printf("%.17g\n", INF);
printf("%.17g\n", -INF);
printf("%f|%e|%g\n", NAN, NAN, NAN);
printf("[%08.2f][%+f]\n", INF, INF);
?>
--EXPECT--
INF
-INF
NaN|NaN|NaN
[INF][INF]
