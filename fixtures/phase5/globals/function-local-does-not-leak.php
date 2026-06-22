<?php
// phase5-runtime: category=globals expect=pass
$x = 1;
function local_scope() {
    $x = 9;
}
local_scope();
echo $x, "\n";
