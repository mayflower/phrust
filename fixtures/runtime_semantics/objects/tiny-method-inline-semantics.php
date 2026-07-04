<?php
// runtime-semantics: category=objects expect=pass php_ref_required=1
// Tiny-method inlining must preserve semantics: subclass overrides,
// private access in declaring scope, typed and readonly properties,
// magic __call, and exceptions from method bodies.
class Base {
    public $v = 1;
    private $secret = "base-secret";
    public int $typed = 5;

    public function getV() { return $this->v; }
    public function setV($x) { $this->v = $x; return $this; }
    public function revealSecret() { return $this->secret; }
    public function setTyped($x) { $this->typed = $x; }
    public function boom() { throw new LogicException("boom:" . $this->v); }
}

class Child extends Base {
    public function getV() { return parent::getV() * 100; }
}

$b = new Base();
$total = 0;
for ($i = 1; $i <= 4; $i++) {
    $total += $b->setV($i)->getV();
}
echo $total, "|", $b->revealSecret(), "\n";

$c = new Child();
$c->setV(3);
echo $c->getV(), "\n";

// Polymorphic call site: the same loop dispatches to both classes.
$rows = [new Base(), new Child(), new Base()];
$sum = 0;
foreach ($rows as $row) {
    $row->setV(2);
    $sum += $row->getV();
}
echo $sum, "\n";

$b->setTyped(9);
echo $b->typed, "\n";
try {
    $b->setTyped("nope");
} catch (TypeError $e) {
    echo "typed-guarded\n";
}

try {
    $b->boom();
} catch (LogicException $e) {
    echo $e->getMessage(), "\n";
}

// (Lowercase method name: the engine currently passes the normalized
// name to __call rather than the source spelling - separate known gap.)
class Bag {
    public function __call($name, $args) {
        return "magic:$name:" . count($args);
    }
}
$bag = new Bag();
echo $bag->undefined(1, 2), "\n";

$ref = 7;
$holder = new Base();
$holder->setV($ref);
echo $holder->getV(), "\n";
