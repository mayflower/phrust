<?php
$a = [];
$a["042"] = "leading";
$a["+42"] = "plus";
$a["-0"] = "minus-zero";
$a[42] = "int";
$a[0] = "zero";
foreach ($a as $key => $value) {
    echo $key, ":", $value, ";";
}
echo "\n";
