<?php
declare(strict_types=1);

$extensions = array_map('strtolower', array_slice($argv, 1));
$selected = array_fill_keys($extensions, true);
$result = [];

foreach (get_defined_functions()['internal'] ?? [] as $function) {
    try {
        $reflection = new ReflectionFunction($function);
        $extension = $reflection->getExtensionName();
    } catch (Throwable) {
        $extension = null;
    }
    $extension = normalize_extension($extension);
    if ($selected !== [] && !isset($selected[$extension])) {
        continue;
    }
    $result[$extension][] = $function;
}

ksort($result, SORT_STRING);
foreach ($result as &$functions) {
    sort($functions, SORT_STRING);
}
unset($functions);

echo json_encode($result, JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES), "\n";

function normalize_extension(null|string|false $extension): string {
    if ($extension === false || $extension === null || $extension === '') {
        return 'core';
    }
    $extension = strtolower($extension);
    return match ($extension) {
        'spl' => 'spl',
        'pcre' => 'pcre',
        'reflection' => 'reflection',
        'standard' => 'standard',
        'json' => 'json',
        'date' => 'date',
        'tokenizer' => 'tokenizer',
        default => $extension,
    };
}
