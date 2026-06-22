<?php
// phase5-runtime: category=known_gaps expect=known_gap known_gap=E_PHP_RUNTIME_SERIALIZATION_PHASE6_GAP
// PHP reference: constructs the object and invokes __wakeup().
class Prompt43WakeupBox
{
    public function __wakeup(): void
    {
        echo "wakeup-hook|";
    }
}
$value = unserialize('O:17:"Prompt43WakeupBox":0:{}');
echo gettype($value), "\n";
