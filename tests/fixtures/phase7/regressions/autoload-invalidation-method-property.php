<?php
spl_autoload_register(function ($class) {
    if (strtolower($class) === 'phase744autobox') {
        require 'tests/fixtures/phase7/regressions/_support/autoload_invalidation_class.php';
    }
});

class Phase744LocalBox {
    public $value = 'L';

    public function value() {
        return $this->value;
    }
}

$object = new Phase744LocalBox();
for ($i = 0; $i < 10; $i++) {
    if ($i === 5) {
        class_exists('Phase744AutoBox', true);
    }
    echo $object->value(), $object->value, '|';
}
echo "\n";
