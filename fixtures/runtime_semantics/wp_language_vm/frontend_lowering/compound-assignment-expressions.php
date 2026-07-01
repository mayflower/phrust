<?php
// runtime-semantics: category=wp_language_vm expect=pass wordpress_error_class=frontend_lowering fixture_id=WP_A_COMPOUND_ASSIGN wp_area=compound_assignment
// Reduced WordPress language/VM fixture: compound assignments evaluate once and return post-assignment values.
function mark($label, $value)
{
    echo $label, "|";
    return $value;
}

function key_name()
{
    echo "key|";
    return "n";
}

$x = 1;
echo ($x += mark("add", 2)), "|";
echo ($x .= mark("concat", "c")), "|";

$arr = ["n" => 5];
echo ($arr[key_name()] += mark("arr", 3)), "|";
echo $arr["n"], "\n";
