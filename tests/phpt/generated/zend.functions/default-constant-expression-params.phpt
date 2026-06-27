--TEST--
Default parameters accept folded constant expressions
--FILE--
<?php
const A = 10;

function defaults($a = 1 + 1, $b = 1 << 2, $c = "foo" . "bar", $d = A * 10) {
    var_dump($a, $b, $c, $d);
}

defaults();
?>
--EXPECT--
int(2)
int(4)
string(6) "foobar"
int(100)
