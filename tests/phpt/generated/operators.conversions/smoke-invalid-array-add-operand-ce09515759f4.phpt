--TEST--
Generated smoke: invalid array arithmetic operand throws TypeError
--DESCRIPTION--
original php-src path: Zend/tests/add_004.phpt
original source hash: ce09515759f4793f36a916b1cde966d041b4277f2433c73a937f8a68ddf443f3
generated timestamp: 20260628T000000Z
generator version: phpt-operators-conversions-v1
reason: reduced invalid operand TypeError regression generated from reference output
--FILE--
<?php
$a = array(1, 2, 3);

try {
    var_dump($a + 5);
} catch (Throwable $e) {
    echo get_class($e), ": ", $e->getMessage(), "\n";
}
--EXPECT--
TypeError: Unsupported operand types: array + int
