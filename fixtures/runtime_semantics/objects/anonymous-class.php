<?php
// runtime-semantics: category=objects expect=pass
// PHP reference: anonymous classes create normal runtime class-like objects.
$object = new class {
    public function label(): string
    {
        return "anonymous";
    }
};

echo $object->label(), "\n";
