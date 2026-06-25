--TEST--
Generated regression: heredoc, nowdoc, and simple interpolation
--DESCRIPTION--
original php-src path: ext/standard/tests/strings/strval_basic.phpt
generated timestamp: 20260625T000000Z
generator version: phpt-standard-strings-v1
reason: reduced heredoc body trimming, nowdoc raw bytes, and simple $var interpolation coverage
--FILE--
<?php
$counter = 3;
$heredoc = <<<TXT
hello
TXT;
$nowdoc = <<<'TXT'
world
TXT;
echo "-- Iteration $counter --\n";
echo $heredoc, $nowdoc, "\n";
?>
--EXPECT--
-- Iteration 3 --
helloworld
