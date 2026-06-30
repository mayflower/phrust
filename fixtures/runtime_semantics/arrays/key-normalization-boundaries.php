<?php
// runtime-semantics: expect=pass
function dump_keys($array) {
    foreach ($array as $key => $value) {
        echo gettype($key), ":", $key, "=", $value, ";";
    }
    echo "\n";
}

$strings = [];
foreach ([
    "0",
    "00",
    "+1",
    "-1",
    "1.0",
    "9223372036854775807",
    "-9223372036854775808",
    "9223372036854775808",
] as $key) {
    $strings[$key] = $key;
}
dump_keys($strings);

$scalars = [];
$scalars[1.0] = "float-one";
$scalars[-1.0] = "float-minus-one";
$scalars[true] = "true";
$scalars[false] = "false";
dump_keys($scalars);
