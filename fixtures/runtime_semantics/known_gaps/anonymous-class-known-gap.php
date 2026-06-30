<?php
// runtime-semantics: category=known_gaps expect=known_gap known_gap=E_PHP_IR_UNSUPPORTED_CLASSLIKE_OBJECT
// PHP reference: anonymous classes create normal runtime class-like objects.
$object = new class {
    public function label(): string
    {
        return "anonymous";
    }
};

echo $object->label(), "\n";
