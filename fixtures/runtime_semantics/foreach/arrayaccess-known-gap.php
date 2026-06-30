<?php
// runtime-semantics: category=foreach expect=pass
class ArrayAccessFixture implements ArrayAccess
{
    public function offsetExists(mixed $offset): bool
    {
        return true;
    }

    public function offsetGet(mixed $offset): mixed
    {
        return "value";
    }

    public function offsetSet(mixed $offset, mixed $value): void
    {
    }

    public function offsetUnset(mixed $offset): void
    {
    }
}
$box = new ArrayAccessFixture();
echo $box["k"], "\n";
