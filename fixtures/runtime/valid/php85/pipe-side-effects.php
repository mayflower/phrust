<?php
function id($x) {
    return $x;
}

$x = 0;
echo ($x = 7) |> id(...), "|", $x, "\n";
