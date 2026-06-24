<?php
// runtime-semantics: category=enums expect=known_gap known_gap=E_PHP_IR_UNSUPPORTED_REFLECTION
enum ReflectableStatus {
    case Ready;
}

$ref = new ReflectionEnum(ReflectableStatus::class);
echo $ref->getName();
