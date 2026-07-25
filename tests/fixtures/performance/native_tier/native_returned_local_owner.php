<?php

function perf_native_returned_local_owner(string $json): array
{
    $decoded = json_decode($json, true);
    return $decoded;
}

$sum = 0;
for ($i = 0; $i < 100; $i++) {
    $sum += perf_native_returned_local_owner('{"value":7}')['value'];
}
echo $sum, "\n";
