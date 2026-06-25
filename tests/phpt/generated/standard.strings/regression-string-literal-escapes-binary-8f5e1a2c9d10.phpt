--TEST--
Generated regression: binary-safe double-quoted string escapes
--DESCRIPTION--
original php-src path: ext/standard/tests/strings/substr.phpt
generated timestamp: 20260625T000000Z
generator version: phpt-standard-strings-v1
reason: reduced double-quoted NUL, hex, and invalid UTF-8 byte literal coverage
--FILE--
<?php
echo bin2hex("\x41\0\xff"), "\n";
echo bin2hex("\x0n1234\x000"), "\n";
?>
--EXPECT--
4100ff
006e313233340030
