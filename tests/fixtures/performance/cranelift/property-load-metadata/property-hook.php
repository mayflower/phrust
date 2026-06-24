<?php
class PerfPropertyMetadataHook {
    public string $name {
        get {
            return "hook";
        }
    }
}

$object = new PerfPropertyMetadataHook();
echo $object->name, "\n";
