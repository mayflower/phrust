<?php
// phase5-runtime: expect=pass
$x = 1;
function phase4_write_global() {
    global $x;
    $x = 2;
}
phase4_write_global();
echo $x;
echo "\n";
