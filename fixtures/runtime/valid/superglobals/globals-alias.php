<?php
// runtime-fixture: expect=pass
$x = 1;
function runtime_write_global() {
    global $x;
    $x = 2;
}
runtime_write_global();
echo $x;
echo "\n";
