<?php
// runtime-semantics: category=wp_language_vm expect=pass wordpress_error_class=frontend_lowering fixture_id=WP_A_NULL_COALESCE_ASSIGN wp_area=null_coalescing_assignment
// Reduced WordPress language/VM fixture: ??= is lazy and returns the stored value for locals, dims, and properties.
function rhs($label, $value)
{
    echo "rhs:$label|";
    return $value;
}

$x = null;
echo ($x ??= rhs("local", "L")), "|";
echo ($x ??= rhs("local-skip", "LS")), "|";

$arr = [];
echo ($arr["k"] ??= rhs("array", "A")), "|";
echo ($arr["k"] ??= rhs("array-skip", "AS")), "|";

$obj = (object) ["p" => null];
echo ($obj->p ??= rhs("object", "O")), "|";
echo ($obj->p ??= rhs("object-skip", "OS")), "\n";
