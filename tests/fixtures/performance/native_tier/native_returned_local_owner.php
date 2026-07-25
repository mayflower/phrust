<?php

function perf_native_returned_local_owner(string $json): array
{
    $decoded = json_decode($json, true);
    return $decoded;
}

function perf_native_reference_alias_result(array $args, array $defaults): array
{
    $parsed =& $args;
    if ($defaults) {
        return array_merge($defaults, $parsed);
    }
    return $parsed;
}

$sum = 0;
for ($i = 0; $i < 100; $i++) {
    $sum += perf_native_returned_local_owner('{"value":7}')['value'];
    $sum += perf_native_reference_alias_result(
        array('value' => 3),
        array('fallback' => 1),
    )['value'];
    $sum += perf_native_reference_alias_result(
        array('value' => 2),
        array(),
    )['value'];
}
echo $sum, "\n";
