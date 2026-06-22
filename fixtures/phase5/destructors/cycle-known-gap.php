<?php
// phase5-runtime: category=destructors expect=known_gap known_gap=E_PHP_RUNTIME_DESTRUCTOR_CYCLE_GC_GAP
class Node {
    public ?Node $peer = null;

    public function __construct(public string $name) {}

    public function __destruct() {
        echo "d:", $this->name, "\n";
    }
}

$a = new Node("a");
$b = new Node("b");
$a->peer = $b;
$b->peer = $a;
unset($a, $b);
echo "after-unset\n";
