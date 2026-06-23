<?php
function phase7_cranelift_side_exit_helper_status(int $a): int
{
    return ($a + 1) * 2;
}

echo phase7_cranelift_side_exit_helper_status(9223372036854775807), "\n";
