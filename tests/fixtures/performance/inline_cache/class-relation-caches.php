<?php
class RelationSmokeBase {
    public function value(): string {
        return 'p';
    }
}

class RelationSmokeChild extends RelationSmokeBase {
}

interface RelationSmokeIface {
    public function iface(): string;
}

class RelationSmokeImpl implements RelationSmokeIface {
    public function iface(): string {
        return 'i';
    }
}

trait RelationSmokeTrait {
    public function traitValue(): string {
        return 't';
    }
}

class RelationSmokeTraitUser {
    use RelationSmokeTrait;
}

class RelationSmokeOverride extends RelationSmokeBase {
    public function value(): string {
        return 'c';
    }
}

final class RelationSmokeFinal {
    final public function value(): string {
        return 'f';
    }
}

spl_autoload_register(function ($name): void {
    include (__DIR__ . '/PerfRegisteredCache.php');
});

$child = new RelationSmokeChild();
$impl = new RelationSmokeImpl();
$trait = new RelationSmokeTraitUser();
$final = new RelationSmokeFinal();

for ($i = 0; $i < 4; $i++) {
    echo ($child instanceof RelationSmokeBase) ? 'T' : 'F';
    echo ($child instanceof RelationSmokeIface) ? 'T' : 'F';
    echo ($impl instanceof RelationSmokeIface) ? 'I' : 'N';
    echo $trait->traitValue();
    echo $final->value();
}

class_exists('PerfRegisteredCache', true);
$autoloaded = new PerfRegisteredCache();
echo ($autoloaded instanceof PerfRegisteredCache) ? 'A' : 'M';

eval('$relationSmokeEpochTouch = 1;');
for ($i = 0; $i < 3; $i++) {
    echo ($child instanceof RelationSmokeBase) ? 'T' : 'F';
}

foreach ([new RelationSmokeBase(), new RelationSmokeOverride(), new RelationSmokeBase(), new RelationSmokeOverride()] as $object) {
    echo $object->value();
}

echo "\n";
