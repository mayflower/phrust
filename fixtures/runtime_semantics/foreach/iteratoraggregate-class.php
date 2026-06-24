<?php
// runtime-semantics: category=foreach expect=pass
class AggregateIteratorFixture implements Iterator
{
    public $items = [3, 4];
    public $pos = 0;

    public function rewind(): void
    {
        $this->pos = 0;
    }

    public function valid(): bool
    {
        return !($this->pos === 2);
    }

    public function current(): mixed
    {
        if ($this->pos === 0) {
            return 3;
        }
        return 4;
    }

    public function key(): mixed
    {
        return $this->pos;
    }

    public function next(): void
    {
        $this->pos = $this->pos + 1;
    }
}

class AggregateFixture implements IteratorAggregate
{
    public function getIterator(): Traversable
    {
        echo "aggregate|";
        return new AggregateIteratorFixture();
    }
}

foreach (new AggregateFixture() as $key => $value) {
    echo $key, ":", $value, ";";
}
echo "\n";
