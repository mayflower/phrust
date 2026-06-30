<?php
// runtime-semantics: expect=fail
function one($value) {
    return $value;
}

one(...["value" => 1], value: 2);
