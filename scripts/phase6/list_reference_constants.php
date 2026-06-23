<?php
declare(strict_types=1);

$extensions = array_map('strtolower', array_slice($argv, 1));
$selected = array_fill_keys($extensions, true);
$result = [];

foreach (get_defined_constants(true) as $extension => $constants) {
    $extension = normalize_extension($extension);
    if ($selected !== [] && !isset($selected[$extension])) {
        continue;
    }
    foreach (array_keys($constants) as $constant) {
        $result[$extension][] = $constant;
    }
}

ksort($result, SORT_STRING);
foreach ($result as &$constants) {
    sort($constants, SORT_STRING);
}
unset($constants);

echo json_encode($result, JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES), "\n";

function normalize_extension(string $extension): string {
    $extension = strtolower($extension);
    return match ($extension) {
        'core' => 'core',
        'standard' => 'standard',
        'spl' => 'spl',
        'pcre' => 'pcre',
        'reflection' => 'reflection',
        'json' => 'json',
        'date' => 'date',
        'tokenizer' => 'tokenizer',
        default => $extension,
    };
}
