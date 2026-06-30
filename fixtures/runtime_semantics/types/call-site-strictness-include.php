<?php
// runtime-semantics: category=types expect=pass
declare(strict_types=1);

include __DIR__ . "/_data/call-site-strict-callee.php";

try {
    binder_strict_callee_takes_int("41");
} catch (TypeError $error) {
    echo "strict-caller\n";
}
