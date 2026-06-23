<?php
function phase7_cranelift_unstable_type_switch(int $a): int
{
    return $a + 1;
}

echo phase7_cranelift_unstable_type_switch(1), "\n";
echo phase7_cranelift_unstable_type_switch("2"), "\n";
echo phase7_cranelift_unstable_type_switch("3"), "\n";
echo phase7_cranelift_unstable_type_switch(4), "\n";
