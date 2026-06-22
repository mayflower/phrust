<?php
// phase5-runtime: expect=fail
function one($value) {
    return $value;
}

one(1, value: 2);
