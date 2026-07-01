<?php
// runtime-semantics: category=wp_language_vm expect=pass wordpress_error_class=runtime_semantics fixture_id=WP_A_TRUTHINESS_MATRIX wp_area=truthiness
// Reduced WordPress language/VM fixture: PHP truthiness is shared by conditions, boolean operators, ternary, elvis, and empty().
$values = [
    "false" => false,
    "zero-int" => 0,
    "zero-float" => 0.0,
    "empty-string" => "",
    "zero-string" => "0",
    "empty-array" => [],
    "null" => null,
    "true" => true,
    "one" => 1,
    "string" => "x",
    "array" => [0],
    "object" => new stdClass(),
];

foreach ($values as $label => $value) {
    $elvis = $value ?: "E";
    echo $label, ":";
    echo $value ? "T" : "F";
    echo ($value && true) ? "T" : "F";
    echo ($value || false) ? "T" : "F";
    echo $elvis === "E" ? "E" : "V";
    echo empty($value) ? ":empty" : ":filled";
    echo "\n";
}
