<?php
function phase7_cranelift_packed_fetch_string_key(array $xs, $key): int {
    return $xs[$key];
}

$xs = ["name" => 42];
echo phase7_cranelift_packed_fetch_string_key($xs, "name"), "\n";
