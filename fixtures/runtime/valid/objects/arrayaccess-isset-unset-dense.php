<?php
// runtime-fixture: kind=valid
// Regression: dense isset/empty/unset arms taking the userland ArrayAccess
// fast path must advance the instruction pointer (a bare `continue` in the
// dense dispatch loop re-executed the same instruction forever).
class Box implements ArrayAccess
{
    private $data = ["k" => 1];

    public function offsetExists($offset): bool
    {
        return isset($this->data[$offset]);
    }

    public function offsetGet($offset): mixed
    {
        return $this->data[$offset] ?? null;
    }

    public function offsetSet($offset, $value): void
    {
        $this->data[$offset] = $value;
    }

    public function offsetUnset($offset): void
    {
        unset($this->data[$offset]);
    }
}

$box = new Box();
echo isset($box["k"]) ? "y" : "n";
echo empty($box["k"]) ? "e" : "f";
unset($box["k"]);
echo isset($box["k"]) ? "y" : "n", "\n";
