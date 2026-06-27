<?php
class ShapeDto {
    public int $value = 1;
}

class ShapePrivate {
    private int $value = 5;

    public function read(): int {
        return $this->value;
    }
}

class ShapeProtectedBase {
    protected int $value = 6;

    public function readBase(): int {
        return $this->value;
    }
}

class ShapeProtectedChild extends ShapeProtectedBase {
    public function readChild(): int {
        return $this->value;
    }
}

class ShapeDynamic {
}

class ShapeMagic {
    public function __get(string $name) {
        return strlen($name);
    }

    public function __set(string $name, $value): void {
    }
}

class ShapeHook {
    public string $name {
        get {
            return "hook";
        }
    }
}

class ShapeTyped {
    public int $value = 9;
}

class ShapePolyA { public int $value = 1; }
class ShapePolyB { public int $value = 2; }
class ShapePolyC { public int $value = 3; }
class ShapePolyD { public int $value = 4; }
class ShapePolyE { public int $value = 5; }

function shape_read_value($object): int {
    return $object->value;
}

$total = 0;
$dto = new ShapeDto();
for ($i = 0; $i < 6; $i++) {
    $dto->value = $dto->value + 1;
    $total += $dto->value;
}

$private = new ShapePrivate();
$protected = new ShapeProtectedChild();
for ($i = 0; $i < 3; $i++) {
    $total += $private->read();
    $total += $protected->readBase();
    $total += $protected->readChild();
}

$dynamic = new ShapeDynamic();
$dynamic->value = 7;
$total += $dynamic->value;

$magic = new ShapeMagic();
$magic->created = 11;
$total += $magic->missing;

$hook = new ShapeHook();
$total += strlen($hook->name);

$typed = new ShapeTyped();
$total += shape_read_value($typed);
$total += shape_read_value($typed);

foreach ([new ShapePolyA(), new ShapePolyB(), new ShapePolyC(), new ShapePolyD(), new ShapePolyE(), new ShapePolyA()] as $object) {
    $total += shape_read_value($object);
}

echo "property-shapes:", $total, "\n";
