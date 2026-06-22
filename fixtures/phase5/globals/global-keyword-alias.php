<?php
// phase5-runtime: category=globals expect=pass
$x = 1;
function bump_global() {
    global $x;
    $x = $x + 2;
}
bump_global();
echo $x, "\n";
