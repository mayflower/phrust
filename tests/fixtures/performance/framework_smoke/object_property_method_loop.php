<?php
class PerfFrameworkCounter {
    public $value = 0;

    public function add($step) {
        $next = $this->value + $step;
        $this->value = $next;
        return $this->value;
    }
}

$counter = new PerfFrameworkCounter();
for ($i = 1; $i <= 5; $i++) {
    echo 'step=', $counter->add($i), ';prop=', $counter->value, "\n";
}
