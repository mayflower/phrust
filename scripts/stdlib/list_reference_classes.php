<?php
declare(strict_types=1);

$extensions = array_map('strtolower', array_slice($argv, 1));
$selected = array_fill_keys($extensions, true);
$result = [];

foreach (array_merge(get_declared_classes(), get_declared_interfaces(), get_declared_traits()) as $class) {
    try {
        $reflection = new ReflectionClass($class);
        $extension = normalize_extension($reflection->getExtensionName());
        if ($selected !== [] && !isset($selected[$extension])) {
            continue;
        }
        $result[$extension][] = [
            'name' => $reflection->getName(),
            'kind' => class_kind($reflection),
        ];
    } catch (Throwable) {
        continue;
    }
}

ksort($result, SORT_STRING);
foreach ($result as &$classes) {
    usort($classes, static fn(array $left, array $right): int => $left['name'] <=> $right['name']);
}
unset($classes);

echo json_encode($result, JSON_PRETTY_PRINT | JSON_UNESCAPED_SLASHES), "\n";

function normalize_extension(null|string|false $extension): string {
    if ($extension === false || $extension === null || $extension === '') {
        return 'core';
    }
    return strtolower($extension);
}

function class_kind(ReflectionClass $reflection): string {
    if ($reflection->isInterface()) {
        return 'interface';
    }
    if ($reflection->isTrait()) {
        return 'trait';
    }
    if (method_exists($reflection, 'isEnum') && $reflection->isEnum()) {
        return 'enum';
    }
    return 'class';
}
