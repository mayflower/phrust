<?php
// runtime-semantics: category=wp_language_vm expect=pass wordpress_error_class=runtime_semantics fixture_id=WP_A_ISSET_EMPTY_COALESCE wp_area=isset_empty_coalesce
// Reduced WordPress language/VM fixture: isset/empty/coalesce tolerate undefined targets and ??= remains lazy.
function rhs($label, $value)
{
    echo "rhs:$label|";
    return $value;
}

$arr = ["null" => null, "zero" => 0];
$obj = (object) ["null" => null, "value" => "yes"];

echo isset($missing) ? "isset-missing" : "unset-missing", "|";
echo isset($arr["null"]) ? "isset-null" : "unset-null", "|";
echo isset($arr["zero"]) ? "isset-zero" : "unset-zero", "|";
echo empty($missing) ? "empty-missing" : "filled-missing", "|";
echo empty($arr["zero"]) ? "empty-zero" : "filled-zero", "|";
echo empty($obj->value) ? "empty-object" : "filled-object", "|";
echo ($missing ?? "fallback"), "|";
echo ($arr["missing"] ?? "array-fallback"), "|";
echo ($obj->missing ?? "object-fallback"), "|";

echo ($arr["new"] ??= rhs("array", "A")), "|";
echo ($arr["new"] ??= rhs("array-skip", "B")), "|";
echo ($obj->null ??= rhs("object", "O")), "|";
echo ($obj->null ??= rhs("object-skip", "P")), "\n";
