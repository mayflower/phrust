<?php
// runtime-semantics: category=wp_language_vm expect=pass wordpress_error_class=runtime_semantics fixture_id=WP_A_BY_REF_ARGUMENTS wp_area=references
// Reduced WordPress language/VM fixture: by-reference parameters mutate locals, dims, object properties, globals, and static locals.
function bang(&$value)
{
    $value .= "!";
    echo $value, "|";
}

class RefBox
{
    public $prop = "P";
}

$local = "L";
bang($local);

$array = ["slot" => "A"];
bang($array["slot"]);

$box = new RefBox();
bang($box->prop);

$wp_language_vm_global = "G";

function global_bang()
{
    global $wp_language_vm_global;
    bang($wp_language_vm_global);
}

global_bang();

function static_bang()
{
    static $value = "S";
    bang($value);
    echo $value, "|";
}

static_bang();
static_bang();
echo $local, "|", $array["slot"], "|", $box->prop, "|", $wp_language_vm_global, "\n";
