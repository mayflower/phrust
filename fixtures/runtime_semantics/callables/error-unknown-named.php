<?php
// runtime-semantics: expect=fail
function one($value) {
    return $value;
}

one(missing: 1);
