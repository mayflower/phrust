<?php
$a = [];
$a[] = "a";
$a[2] = "c";
unset($a[2]);
$a[] = "d";
foreach ($a as $key => $value) {
    echo $key, ":", $value, ";";
}
echo "\n";
