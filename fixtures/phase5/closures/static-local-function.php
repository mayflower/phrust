<?php
// expect=skip
function next_id() {
    static $x = 0;
    $x++;
    return $x;
}

echo next_id(), '|', next_id();
