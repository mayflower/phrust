<?php
// runtime-semantics: category=gc expect=known_gap known_gap=E_PHP_RUNTIME_GC_PUBLIC_API_GAP
class D {
    public ?D $peer = null;
    public function __construct(public string $name) {}
    public function __destruct() {
        echo "d:", $this->name, "\n";
    }
}

$a = new D("a");
$b = new D("b");
$a->peer = $b;
$b->peer = $a;
unset($a, $b);
echo gc_collect_cycles(), "\n";
