<?php

namespace Exprs;

use function strlen;

$result = strlen(trim(" value "));
$item = $items[0]["name"];
$property = $object->property;
$method = $object?->method($item);
$static = Example::factory()::class;
$dynamic = $class::method(...$args);
