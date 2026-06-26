--TEST--
Generated standard.strings: quotemeta escapes regex metacharacters
--DESCRIPTION--
module: standard.strings
generated timestamp: 20260626T000000Z
generator version: phpt-standard-strings-v1
reason: quotemeta backslash-escapes . \ + * ? [ ^ ] $ ( ) and passes other bytes through, returning "" for the empty string
--FILE--
<?php
var_dump(quotemeta("1+1=2"));
var_dump(quotemeta("a.b\\c+d*e?f[g^h]i\$j(k)l"));
var_dump(quotemeta(""));
var_dump(quotemeta("no specials"));
?>
--EXPECT--
string(6) "1\+1=2"
string(34) "a\.b\\c\+d\*e\?f\[g\^h\]i\$j\(k\)l"
string(0) ""
string(11) "no specials"
