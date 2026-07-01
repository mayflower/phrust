<?php
// runtime-semantics: category=wp_language_vm expect=pass wordpress_error_class=frontend_lowering fixture_id=WP_A_ASSIGN_EXPR wp_area=assignment_expr
// Reduced WordPress language/VM fixture: assignment expressions return the assigned value in expression position.
function mark($label, $value)
{
    echo $label, "|";
    return $value;
}

if ($x = mark("if", 1)) {
    echo "if:$x|";
}

$y = ($x = mark("nested", 2));
echo "x:$x,y:$y|";

function take($arg)
{
    echo "arg:$arg|";
}

take($z = mark("call", 3));
echo $w = "echo";
echo "|";
$a = $b = mark("chain", 4);
echo "a:$a,b:$b\n";
