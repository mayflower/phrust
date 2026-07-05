<?php

function counts(): Generator {
    yield 1;
    yield 2;
    return 'finished';
}

class Feed {
    private array $items = ['a', 'b'];

    public function stream(): \Generator {
        foreach ($this->items as $item) {
            yield $item;
        }
        return count($this->items);
    }
}

$g = counts();
foreach ($g as $v) {
    echo $v, ',';
}
var_dump($g->getReturn());

$feed = new Feed();
$s = $feed->stream();
foreach ($s as $v) {
    echo $v, ',';
}
var_dump($s->getReturn());

function silent(): Generator {
    yield 9;
}
$q = silent();
foreach ($q as $v) {
    echo $v, "\n";
}
var_dump($q->getReturn());
