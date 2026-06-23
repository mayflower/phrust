<?php
function phase7_cranelift_concat_template_piece(string $lhs, string $rhs): string {
    return $lhs . $rhs;
}

$out = "";
for ($i = 0; $i < 64; $i++) {
    $out = phase7_cranelift_concat_template_piece($out, "x");
}

echo strlen($out), "\n";
