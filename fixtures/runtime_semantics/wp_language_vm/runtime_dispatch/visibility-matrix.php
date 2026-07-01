<?php
// runtime-semantics: category=wp_language_vm expect=pass wordpress_error_class=runtime_dispatch fixture_id=WP_A_VISIBILITY_MATRIX wp_area=visibility
// Reduced WordPress language/VM fixture: member visibility is caller-sensitive and private methods bind to the declaring class.
class VisibilityBase
{
    public function publicLabel()
    {
        return "public";
    }

    protected function protectedLabel()
    {
        return "protected";
    }

    private function privateLabel()
    {
        return "private-base";
    }

    public function baseInside()
    {
        return $this->publicLabel() . "|" . $this->protectedLabel() . "|" . $this->privateLabel();
    }

    public function baseCallsChild(VisibilityChild $child)
    {
        return $child->protectedLabel();
    }
}

class VisibilityChild extends VisibilityBase
{
    private function childPrivate()
    {
        return "private-child";
    }

    public function childInside()
    {
        return $this->protectedLabel() . "|" . $this->childPrivate();
    }
}

function attempt($label, $callback)
{
    try {
        echo $label, ":", $callback(), "\n";
    } catch (Error $error) {
        echo $label, ":Error\n";
    }
}

$child = new VisibilityChild();
attempt("public", function () use ($child) { return $child->publicLabel(); });
attempt("base-inside", function () use ($child) { return $child->baseInside(); });
attempt("child-inside", function () use ($child) { return $child->childInside(); });
attempt("parent-protected-child", function () use ($child) { return (new VisibilityBase())->baseCallsChild($child); });
attempt("outside-protected", function () use ($child) { return $child->protectedLabel(); });
attempt("outside-private", function () use ($child) { return $child->childPrivate(); });
