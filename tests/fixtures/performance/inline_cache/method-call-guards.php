<?php
final class MethodFinalLoop {
    public int $value = 3;
    final public function value(): int {
        return $this->value;
    }
}

class MethodNormalLoop {
    public function value(): int {
        return 4;
    }
}

class MethodPolyBase {
    public function value(): string {
        return 'A';
    }
}

class MethodPolyChild extends MethodPolyBase {
    public function value(): string {
        return 'B';
    }
}

class MethodMagicCall {
    public function __call($name, $args): string {
        return $name . count($args);
    }
}

class MethodVisibilityBase {
    private function secret(): string {
        return 's';
    }

    protected function protectedValue(): string {
        return 'p';
    }

    public function callSecret(): string {
        return $this->secret();
    }
}

class MethodVisibilityChild extends MethodVisibilityBase {
    public function callProtected(): string {
        return $this->protectedValue();
    }
}

class MethodArgsFallback {
    public function named($a, $b): string {
        return $a . $b;
    }
}

class MethodThrows {
    public function fail(): void {
        throw new Exception('method-boom');
    }
}

final class MethodDto {
    public int $amount;

    public function __construct(int $amount) {
        $this->amount = $amount;
    }

    final public function amount(): int {
        return $this->amount;
    }
}

final class MethodService {
    final public function total(MethodDto $dto): int {
        return $dto->amount();
    }
}

$final = new MethodFinalLoop();
$sum = 0;
for ($i = 0; $i < 8; $i++) {
    $sum += $final->value();
}
echo 'final=', $sum, "\n";

$normal = new MethodNormalLoop();
$sum = 0;
for ($i = 0; $i < 8; $i++) {
    $sum += $normal->value();
}
echo 'normal=', $sum, "\n";

$poly = [new MethodPolyBase(), new MethodPolyChild(), new MethodPolyBase(), new MethodPolyChild()];
foreach ($poly as $object) {
    echo $object->value();
}
echo "\n";

$magic = new MethodMagicCall();
echo $magic->missing(1, 2), "\n";

$visible = new MethodVisibilityChild();
echo $visible->callSecret(), $visible->callProtected(), "\n";

$args = new MethodArgsFallback();
echo 'named=', $args->named(b: 'B', a: 'A'), "\n";

try {
    (new MethodThrows())->fail();
} catch (Exception $exception) {
    echo 'caught=', $exception->getMessage(), "\n";
}

$service = new MethodService();
$sum = 0;
for ($i = 0; $i < 6; $i++) {
    $sum += $service->total(new MethodDto($i));
}
echo 'dto=', $sum, "\n";
