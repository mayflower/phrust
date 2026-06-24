<?php
spl_autoload_register(function ($class) {
    if (strtolower($class) === 'performance44autobox') {
        require 'tests/fixtures/performance/regressions/_support/autoload_invalidation_class.php';
    }
});

class PerfRegressionLocalBox {
    public $value = 'L';

    public function value() {
        return $this->value;
    }
}

$object = new PerfRegressionLocalBox();
for ($i = 0; $i < 10; $i++) {
    if ($i === 5) {
        class_exists('PerfRegressionAutoBox', true);
    }
    echo $object->value(), $object->value, '|';
}
echo "\n";
