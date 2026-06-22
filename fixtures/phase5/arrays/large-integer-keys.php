<?php
$a = [];
$a[9223372036854775806] = "x";
$a[] = "y";
foreach ($a as $key => $value) {
    echo $key, ":", $value, ";";
}
echo "\n";
