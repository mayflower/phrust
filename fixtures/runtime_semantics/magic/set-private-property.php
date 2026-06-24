<?php
class MagicSetPrivate {
    private string $secret = "";
    public string $seen = "";

    public function __set(string $name, mixed $value): void {
        $this->seen = $name . "=" . $value;
    }
}

$object = new MagicSetPrivate();
$object->secret = "value";
echo $object->seen;
