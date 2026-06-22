<?php
// phase5-runtime: expect=fail
function pair($first, $second) {
    return $first + $second;
}

pair(first: 1, 2);
