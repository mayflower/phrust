<?php
// runtime-semantics: category=reflection expect=known_gap known_gap=E_PHP_VM_REFLECTION_UNSUPPORTED_CALLABLE
class ReflectionCallableTarget {
    public static function wrap(): string {
        return "wrapped";
    }
}

$callable = ReflectionCallableTarget::wrap(...);
$function = new ReflectionFunction($callable);
echo $function->getName(), "\n";
