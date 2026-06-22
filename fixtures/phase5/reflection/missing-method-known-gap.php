<?php
// phase5-runtime: category=reflection expect=known_gap known_gap=E_PHP_VM_UNKNOWN_METHOD
class ReflectionMissingMethodTarget
{
}

echo (new ReflectionClass(ReflectionMissingMethodTarget::class))->getConstructor();
