<?php
$a = [];
$a["42"] = "string-int";
$a[42] = "int-overwrite";
$a["-42"] = "negative-string-int";
$a["-42"] = "negative-int-overwrite";
$a["0"] = "zero-string";
$a[0] = "zero-int-overwrite";
foreach ($a as $key => $value) {
    echo $key, ":", $value, ";";
}
echo "\n";
