<?php
// runtime-semantics: category=known_gaps expect=known_gap known_gap=E_PHP_RUNTIME_SERIALIZATION_STDLIB_GAP
// PHP reference: constructs the object and invokes __wakeup().
class WakeupBoxFixture
{
    public function __wakeup(): void
    {
        echo "wakeup-hook|";
    }
}
$value = unserialize('O:17:"WakeupBoxFixture":0:{}');
echo gettype($value), "\n";
