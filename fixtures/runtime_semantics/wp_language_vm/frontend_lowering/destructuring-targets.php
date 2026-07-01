<?php
// runtime-semantics: category=wp_language_vm expect=pass wordpress_error_class=frontend_lowering fixture_id=WP_A_DESTRUCTURING_TARGETS wp_area=destructuring
// Reduced WordPress language/VM fixture: string-key, nested, array-dim, and property destructuring targets assign left-to-right.
$row = [
    "name" => "ada",
    "pair" => ["left", "right"],
    "slot" => "array-target",
    "prop" => "property-target",
];

$target = [];
$box = new stdClass();

[
    "name" => $name,
    "pair" => [$left, $right],
    "slot" => $target["slot"],
    "prop" => $box->p,
] = $row;

echo $name, "|", $left, "|", $right, "|", $target["slot"], "|", $box->p, "\n";
