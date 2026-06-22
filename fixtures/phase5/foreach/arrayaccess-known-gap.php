<?php
// phase5-runtime: category=foreach expect=known_gap known_gap=E_PHP_RUNTIME_ARRAYACCESS_PHASE6_GAP
class Prompt42ArrayAccess implements ArrayAccess
{
    public function offsetExists($offset)
    {
        return true;
    }

    public function offsetGet($offset)
    {
        return "value";
    }

    public function offsetSet($offset, $value)
    {
    }

    public function offsetUnset($offset)
    {
    }
}
$box = new Prompt42ArrayAccess();
echo $box["k"], "\n";
