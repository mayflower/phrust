<?php
class Phase7PropertyMetadataHook {
    public string $name {
        get {
            return "hook";
        }
    }
}

$object = new Phase7PropertyMetadataHook();
echo $object->name, "\n";
