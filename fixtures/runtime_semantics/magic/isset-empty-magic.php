<?php
class MagicIssetEmpty {
    public function __isset(string $name): bool {
        return $name === "present";
    }

    public function __get(string $name): string {
        return ($name === "present") ? "0" : "value";
    }
}

$object = new MagicIssetEmpty();
echo isset($object->present) ? "isset" : "no";
echo "|";
echo empty($object->present) ? "empty" : "not-empty";
echo "|";
echo isset($object->missing) ? "isset" : "no";
echo "|";
echo empty($object->missing) ? "empty" : "not-empty";
