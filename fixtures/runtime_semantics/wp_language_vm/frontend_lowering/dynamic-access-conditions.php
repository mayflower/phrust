<?php
// runtime-semantics: category=wp_language_vm expect=pass wordpress_error_class=frontend_lowering fixture_id=WP_A_DYNAMIC_ACCESS wp_area=dynamic_access
// Reduced WordPress language/VM fixture: dynamic receivers, keys, properties, and method names are evaluated once.
class Box
{
    public $ready = "yes";

    public function run($value)
    {
        echo "run:$value|";
        return "done";
    }
}

function receiver($object)
{
    echo "receiver|";
    return $object;
}

function prop_name()
{
    echo "prop|";
    return "ready";
}

function key_name()
{
    echo "key|";
    return "item";
}

function method_name()
{
    echo "method|";
    return "run";
}

$box = new Box();
if (receiver($box)->{prop_name()}) {
    echo "object-truth|";
}

$items = ["item" => $box];
$prop = prop_name();
if ($items[key_name()]->$prop) {
    echo "array-truth|";
}

echo $box->{method_name()}("arg"), "\n";
