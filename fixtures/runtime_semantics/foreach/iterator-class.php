<?php
// runtime-semantics: category=foreach expect=pass
class IteratorFixture implements Iterator
{
    public $items = [10, 20];
    public $pos = 0;

    public function rewind(): void
    {
        echo "rewind|";
        $this->pos = 0;
    }

    public function valid(): bool
    {
        echo "valid|";
        return !($this->pos === 2);
    }

    public function current(): mixed
    {
        echo "current|";
        if ($this->pos === 0) {
            return 10;
        }
        return 20;
    }

    public function key(): mixed
    {
        echo "key|";
        return $this->pos;
    }

    public function next(): void
    {
        echo "next|";
        $this->pos = $this->pos + 1;
    }
}
foreach (new IteratorFixture() as $key => $value) {
    echo "body:", $key, "=", $value, "|";
}
echo "done\n";
