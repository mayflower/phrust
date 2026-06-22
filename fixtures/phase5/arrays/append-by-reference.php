<?php
// phase5-runtime: expect=pass
$a = [];
$b = 2;
$a[] =& $b;
$b = 5;
echo $a[0];
