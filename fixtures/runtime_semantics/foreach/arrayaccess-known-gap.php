<?php
// runtime-semantics: category=foreach expect=known_gap known_gap=E_PHP_RUNTIME_ARRAYACCESS_STDLIB_GAP
class ArrayAccessFixture implements ArrayAccess
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
$box = new ArrayAccessFixture();
echo $box["k"], "\n";
