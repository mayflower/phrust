<?php
// phase5-runtime: expect=fail
function one($value) {
    return $value;
}

one(missing: 1);
