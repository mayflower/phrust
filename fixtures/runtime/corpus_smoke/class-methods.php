<?php
// phase4-runtime: corpus=pass
class CorpusCounter
{
    public int $count;

    public function __construct(int $start)
    {
        $this->count = $start;
    }

    public function increment(int $by): int
    {
        $this->count = $this->count + $by;
        return $this->count;
    }

    public function label(string $name): string
    {
        return $name;
    }
}

$counter = new CorpusCounter(1);
echo $counter->count, "|", $counter->increment(2), "|", $counter->label("jobs"), "\n";
