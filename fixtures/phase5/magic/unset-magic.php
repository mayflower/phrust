<?php
class MagicUnset {
    public string $seen = "";

    public function __unset(string $name): void {
        $this->seen = "unset:" . $name;
    }
}

$object = new MagicUnset();
unset($object->missing);
echo $object->seen;
