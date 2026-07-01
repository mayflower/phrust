<?php
// runtime-semantics: category=wp_language_vm expect=pass wordpress_error_class=runtime_dispatch fixture_id=WP_A_MAGIC_CALL wp_area=magic_call
// Reduced WordPress language/VM fixture: __call handles missing and inaccessible instance methods without replacing accessible methods.
class MagicBox
{
    public function real($value)
    {
        return "real:$value";
    }

    private function hidden($value)
    {
        return "hidden:$value";
    }

    public function __call($name, $args)
    {
        echo "__call:$name:", count($args), ":", $args[0], "\n";
    }
}

$box = new MagicBox();
echo $box->real("ok"), "\n";
$box->missing("a", "b");
$box->hidden("secret");
