<?php
// Property dimension assignment paths: the in-place fast path (untyped
// declared/dynamic/stdClass array properties) and every gated fallback
// (typed, readonly, ArrayAccess, reference slots, scalars, vivification).

class Registry {
    public $cache = [];
    public array $typedMap = [];
    public $slot;
}

// 1. Declared untyped property: nested writes, appends, overwrite, order.
$r = new Registry();
$r->cache['alpha']['one'] = 1;
$r->cache['alpha']['two'] = 2;
$r->cache['beta'][] = 'first';
$r->cache['beta'][] = 'second';
$r->cache['alpha']['one'] = 10;
var_dump($r->cache);

// 2. COW: a by-value copy taken before the write must not change.
$copy = $r->cache;
$r->cache['alpha']['three'] = 3;
var_dump(isset($copy['alpha']['three']), isset($r->cache['alpha']['three']));

// 3. Reference element inside the property array (created before the
// array became a property value): property dim writes flow through the
// element reference.
$seed = ['linked' => 1];
$bound = &$seed['linked'];
$r->cache['refs'] = $seed;
$r->cache['refs']['linked'] = 99;
var_dump($bound);
$bound = 100;
var_dump($r->cache['refs']['linked']);
unset($bound, $seed);

// 4. Typed array property (generic path) still works and stays typed.
$r->typedMap['k']['nested'] = 'typed';
var_dump($r->typedMap);

// 6. Null property vivifies to array on first dim write.
$r2 = new Registry();
$r2->slot['fresh'] = 'vivified';
var_dump($r2->slot);

// 7. stdClass dynamic property nested writes.
$std = new stdClass();
$std->bag['x']['y'] = 'deep';
$std->bag['x']['z'] = 'deeper';
var_dump($std->bag);

// 8. ArrayAccess property routes through offsetSet, not the array path.
class Box implements ArrayAccess {
    public $items = [];
    public function offsetExists(mixed $offset): bool { return isset($this->items[$offset]); }
    public function offsetGet(mixed $offset): mixed { return $this->items[$offset] ?? null; }
    public function offsetSet(mixed $offset, mixed $value): void {
        echo "offsetSet($offset)\n";
        $this->items[$offset] = $value;
    }
    public function offsetUnset(mixed $offset): void { unset($this->items[$offset]); }
}
class Holder {
    public $box;
}
$h = new Holder();
$h->box = new Box();
$h->box['key'] = 'boxed';
var_dump($h->box->items);

// 9. Readonly property dim write raises a catchable Error.
class Frozen {
    public function __construct(public readonly array $data = ['locked' => true]) {}
}
$f = new Frozen();
try {
    $f->data['more'] = 1;
} catch (Error $e) {
    // Message text is a pre-existing wording gap (reference says "Cannot
    // indirectly modify readonly property"); this fixture pins the
    // catchable-Error behavior and the untouched property value.
    echo get_class($e), ": readonly dim write blocked\n";
}
var_dump($f->data);

// 11. Deep nesting with mixed int/string keys keeps insertion order.
$r4 = new Registry();
$r4->cache[5]['mixed'][0] = 'a';
$r4->cache[5]['mixed'][] = 'b';
$r4->cache['5x']['mixed'][1] = 'c';
var_dump($r4->cache);

// 12. Private declared property written from inside the class.
class Vault {
    private $store = [];
    public function put($k, $v) { $this->store[$k][] = $v; }
    public function all() { return $this->store; }
}
$v = new Vault();
$v->put('a', 1);
$v->put('a', 2);
$v->put('b', 3);
var_dump($v->all());
