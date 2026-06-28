<?php
function bytecode_method_call_target($object) {
    return $object->value();
}

echo "method-call-fenced\n";
