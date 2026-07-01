<?php
// runtime-semantics: category=enums expect=pass
enum ReflectableStatus {
    case Ready;
}

$ref = new ReflectionEnum(ReflectableStatus::class);
echo $ref->getName();
