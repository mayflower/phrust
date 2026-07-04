<?php
// runtime-semantics: category=includes expect=pass php_ref_required=1
// Argument coercion follows the CALL SITE's strict_types mode, not the
// declaring file's: this weak-mode file calls methods and functions
// declared in a strict_types=1 include, so numeric strings and ints
// coerce per weak rules on every dispatch shape (instance, static,
// function), repeated in a loop so inline-cached dense dispatch is hot.

require __DIR__ . '/_data/strict-typed-service.php';

$service = new StrictTypedService();
$total = 0;
for ($i = 0; $i < 4; $i++) {
    $total += $service->takesInt("5");
    $total += $service->takesInt(2.0);
    $total += StrictTypedService::scaled("3");
}
echo $total, "\n";
echo strict_takes_float("7"), "|", strict_takes_float(9), "\n";

// Non-numeric strings still fail under weak rules.
try {
    $service->takesInt("nope");
} catch (TypeError $e) {
    echo "weak-rejects-non-numeric\n";
}
