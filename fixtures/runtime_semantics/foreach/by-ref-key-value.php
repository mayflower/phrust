<?php
$items = ["a" => 1, "b" => 2];
foreach ($items as $key => &$value) {
    echo $key, ":", $value, ";";
    $value = $value + 1;
}
unset($value);
echo "|", $items["a"], ":", $items["b"], "\n";
