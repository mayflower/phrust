<?php

declare(strict_types=1);

namespace Statements;

global $globalName;
static $counter = 0;
echo $counter;
unset($globalName);

start:
goto after_start;
after_start:

if ($counter > 0) {
    echo $counter;
} else {
    echo 0;
}

while ($counter < 2) {
    $counter++;
    continue;
}

do {
    $counter--;
} while ($counter > 0);

for ($i = 0; $i < 2; $i++) {
    if ($i === 1) {
        break;
    }
}

foreach ([1, 2] as $key => $value) {
    echo $key, $value;
}

switch ($counter) {
    case 0:
        echo "zero";
        break;
    default:
        echo "other";
}

try {
    throw new \Exception("test");
} catch (\Exception $e) {
    echo $e->getMessage();
} finally {
    echo "done";
}

return $counter;
