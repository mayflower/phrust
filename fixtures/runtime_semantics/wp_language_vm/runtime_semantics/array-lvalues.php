<?php
// runtime-semantics: category=wp_language_vm expect=pass wordpress_error_class=runtime_semantics fixture_id=WP_A_ARRAY_LVALUES wp_area=array_lvalues
// Reduced WordPress language/VM fixture: array lvalues normalize common keys, append, auto-create nested dims, unset, and support destructuring reads.
$array = [];
$array[] = "append";
$array[true] = "bool";
$array["2"] = "numeric-string";
$array["deep"]["leaf"] = "nested";
unset($array["missing"]);

$row = ["name" => "row", "items" => ["zero", "one"]];
["name" => $name, "items" => [1 => $array["picked"]]] = $row;

echo $array[0], "|", $array[1], "|", $array[2], "|";
echo $array["deep"]["leaf"], "|", $name, "|", $array["picked"], "|";
echo isset($array["missing"]) ? "missing-key-present" : "missing-key-absent";
echo "\n";
