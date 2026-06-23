<?php
class Phase7Counter {
    public $value = 0;

    public function add($step) {
        $this->value = $this->value + $step;
        return $this->value;
    }
}

$counter = new Phase7Counter();
$last = 0;
for ($i = 1; $i <= 4; $i++) {
    $last = $counter->add($i);
}
echo "methods:", $last, "\n";
