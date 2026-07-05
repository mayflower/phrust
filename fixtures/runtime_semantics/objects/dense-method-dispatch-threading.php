<?php

class Base {
    protected int $count = 0;

    final public function bump(int $step): int {
        $this->count += $step;
        return $this->count;
    }

    public function describe(): string {
        return 'base:' . $this->count;
    }

    public static function label(): string {
        return 'Base';
    }

    private function secret(): string {
        return 'hidden';
    }

    public function reveal(): string {
        return $this->secret();
    }
}

class Derived extends Base {
    public function describe(): string {
        return 'derived:' . parent::describe();
    }
}

class Magic {
    public function __call(string $name, array $args): string {
        return "magic:$name(" . implode(',', $args) . ")";
    }
}

class Thrower {
    public function boom(string $what): never {
        throw new RuntimeException("boom:$what");
    }
}

$base = new Base();
$derived = new Derived();

for ($i = 0; $i < 12; $i++) {
    $base->bump(1);
    $derived->bump(2);
}
var_dump($base->bump(0), $derived->bump(0));

$receivers = [$base, $derived, $base, $derived];
$out = [];
foreach ($receivers as $receiver) {
    $out[] = $receiver->describe();
}
var_dump($out);

var_dump(Base::label(), Derived::label());
var_dump($base->reveal());

$magic = new Magic();
var_dump($magic->missing(1, 'two'));

$thrower = new Thrower();
try {
    $thrower->boom('caught');
} catch (RuntimeException $error) {
    var_dump($error->getMessage());
}

try {
    $base->secret();
} catch (Error $error) {
    var_dump($error->getMessage());
}
