<?php
class PerfRegressionTemp {
    public $label;

    public function __construct($label) {
        $this->label = $label;
    }

    public function __destruct() {
        echo 'D', $this->label, '|';
        if ($this->label === 0) {
            echo "\n";
        }
    }
}

for ($i = 0; $i < 4; $i++) {
    $tmp = new PerfRegressionTemp($i);
    echo $i + 1, '|';
    unset($tmp);
}
echo "\n";
