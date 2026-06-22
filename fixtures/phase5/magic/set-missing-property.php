<?php
class MagicSetMissing {
    public string $seen = "";

    public function __set(string $name, mixed $value): void {
        $this->seen = $name . "=" . $value;
    }
}

$object = new MagicSetMissing();
$object->missing = "value";
echo $object->seen;
