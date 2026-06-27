<?php
function add_named($left, $right) {
    return $left + $right;
}

function mutate_value(&$value) {
    $value += 2;
    return $value;
}

$name = "add_named";
$total = 0;
for ($i = 0; $i < 4; $i++) {
    $total += add_named(1, 2);
    $total += add_named(left: 1, right: 2);
    $total += $name(2, 3);
}
$value = 1;
$total += mutate_value($value);
echo "call-guards:", $total, ":", $value, "\n";
