<?php

function bump(int &$n, int $step): void {
    $n += $step;
}

function push_item(array &$items, string $item): int {
    $items[] = $item;
    return count($items);
}

function reads_only(array &$items): int {
    return count($items);
}

function creates_missing(&$slot): void {
    $slot = 'created';
}

function into_dim(array &$rows): void {
    $rows[] = 'row';
}

function into_property(array &$bag): void {
    $bag['tag'] = 'set';
}

class Holder {
    public array $bag = ['seed'];
}

$counter = 40;
bump($counter, 2);
var_dump($counter);

$list = ['a'];
var_dump(push_item($list, 'b'));
var_dump($list);

$shared = ['x' => 1];
$copy = $shared;
var_dump(reads_only($shared));
var_dump($shared === $copy);

creates_missing($fresh);
var_dump($fresh);

$matrix = ['rows' => []];
into_dim($matrix['rows']);
var_dump($matrix['rows']);

$holder = new Holder();
into_property($holder->bag);
var_dump($holder->bag);

bump(n: $counter, step: 3);
var_dump($counter);

$callable = 'bump';
$counter2 = 5;
$callable($counter2, 10);
var_dump($counter2);

var_dump(call_user_func('bump', 7, 1));
var_dump($counter);
