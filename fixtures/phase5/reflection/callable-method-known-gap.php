<?php
// phase5-runtime: category=reflection expect=known_gap known_gap=E_PHP_VM_REFLECTION_UNSUPPORTED_CALLABLE
class Prompt28CallableTarget {
    public static function wrap(): string {
        return "wrapped";
    }
}

$callable = Prompt28CallableTarget::wrap(...);
$function = new ReflectionFunction($callable);
echo $function->getName(), "\n";
