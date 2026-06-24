<?php
// runtime-semantics: expect=fail
function one($value) {
    return $value;
}

one(...1);
