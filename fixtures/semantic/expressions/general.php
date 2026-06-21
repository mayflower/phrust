<?php

namespace Exprs;

$a = 1;
$b = -$a + 2 * 3;
$c = $a ? $b : null;
$d = (string) $c;
$e = !$d;
$f = $a ?? 0;
