<?php
function coerce_ref(int &$value): void {
    echo gettype($value), ":", $value, "|";
}

$value = "42";
coerce_ref($value);
echo gettype($value), ":", $value;
