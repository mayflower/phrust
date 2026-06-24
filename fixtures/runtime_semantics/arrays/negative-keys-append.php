<?php
$a = [];
$a["-5"] = "a";
$a[] = "b";
foreach ($a as $key => $value) {
    echo $key, ":", $value, ";";
}
echo "\n";
