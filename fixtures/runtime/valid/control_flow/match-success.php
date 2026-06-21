<?php
// phase4: kind=valid expected_stdout="one\n"
$x = 1;
echo match ($x) {
    0 => "zero",
    1 => "one",
    default => "default",
};
echo "\n";
