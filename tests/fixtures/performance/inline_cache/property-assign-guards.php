<?php
class AssignDto {
    public int $value = 0;
}

class AssignPrivate {
    private int $value = 0;

    public function hydrate(): int {
        for ($i = 0; $i < 6; $i++) {
            $this->value = $i;
        }

        return $this->value;
    }
}

class AssignProtectedBase {
    protected int $value = 1;

    public function setBase(int $value): void {
        $this->value = $value;
    }

    public function readBase(): int {
        return $this->value;
    }
}

class AssignProtectedChild extends AssignProtectedBase {
    public function setChild(int $value): void {
        $this->value = $value;
    }

    public function readChild(): int {
        return $this->value;
    }
}

class AssignTyped {
    public int $value = 0;
}

class AssignDynamic {
}

class AssignMagic {
    public int $seen = 0;

    public function __set(string $name, $value): void {
        $this->seen = $value;
    }
}

class AssignHook {
    public string $name {
        set {
            $this->name = strtoupper($value);
        }
        get {
            return $this->name;
        }
    }
}

$total = 0;
$dto = new AssignDto();
for ($i = 0; $i < 8; $i++) {
    $dto->value = $i;
    $total += $dto->value;
}

$private = new AssignPrivate();
$total += $private->hydrate();
try {
    $private->value = 5;
} catch (Throwable $e) {
    $total += 5;
}

$protected = new AssignProtectedChild();
$protected->setBase(3);
$protected->setChild(4);
$total += $protected->readBase();
$total += $protected->readChild();

$typed = new AssignTyped();
$typed->value = 9;
$total += $typed->value;
try {
    $typed->value = 'bad';
} catch (Throwable $e) {
    $total += 1;
}

$dynamic = new AssignDynamic();
$dynamic->value = 6;
$total += $dynamic->value;

$magic = new AssignMagic();
$magic->missing = 4;
$total += $magic->seen;

$hook = new AssignHook();
$hook->name = "ada";
$total += strlen($hook->name);

echo "property-assign:", $total, "\n";
