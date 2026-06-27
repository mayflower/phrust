<?php
$packed = array(1, 2, 3, 4);
$sum = 0;
foreach ($packed as $value) {
    $sum += $value;
}

$mixed = array(
    'first' => 'alpha',
    'second' => 'beta',
    8 => 'gamma',
);

echo 'packed=', count($packed), ':', $sum, "\n";
foreach ($mixed as $key => $value) {
    echo $key, '=', strtoupper($value), "\n";
}
