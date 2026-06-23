<?php
class Phase744FiberBox {
    public $value = 3;

    public function value() {
        return $this->value;
    }
}

$fiber = new Fiber(function () {
    $box = new Phase744FiberBox();
    for ($i = 0; $i < 12; $i++) {
        echo $box->value(), $box->value, '|';
        if ($i === 5) {
            Fiber::suspend('pause');
        }
    }
});

echo $fiber->start(), '|';
$fiber->resume(null);
echo 'done';
echo "\n";
