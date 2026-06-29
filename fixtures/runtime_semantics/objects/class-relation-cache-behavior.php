<?php
class RelationFixtureBase {
    public function value(): string {
        return 'p';
    }
}

class RelationFixtureChild extends RelationFixtureBase {
}

interface RelationFixtureIface {
    public function iface(): string;
}

class RelationFixtureImpl implements RelationFixtureIface {
    public function iface(): string {
        return 'i';
    }
}

trait RelationFixtureTrait {
    public function traitValue(): string {
        return 't';
    }
}

class RelationFixtureTraitUser {
    use RelationFixtureTrait;
}

class RelationFixtureOverride extends RelationFixtureBase {
    public function value(): string {
        return 'c';
    }
}

final class RelationFixtureFinal {
    final public function value(): string {
        return 'f';
    }
}

$child = new RelationFixtureChild();
$impl = new RelationFixtureImpl();
$trait = new RelationFixtureTraitUser();
$final = new RelationFixtureFinal();

echo ($child instanceof RelationFixtureBase) ? "extends=yes\n" : "extends=no\n";
echo ($child instanceof RelationFixtureIface) ? "negative=no\n" : "negative=yes\n";
echo ($impl instanceof RelationFixtureIface) ? "implements=yes\n" : "implements=no\n";
echo "trait=", $trait->traitValue(), "\n";
echo "override=", (new RelationFixtureOverride())->value(), "\n";
echo "final=", $final->value(), "\n";
eval('$relationFixtureEpochTouch = 1;');
echo ($child instanceof RelationFixtureBase) ? "after-eval=yes\n" : "after-eval=no\n";
