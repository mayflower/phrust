<?php
class PerfRegressionAliasBox {
    public $value = 1;

    public function bump() {
        $this->value = $this->value + 1;
    }
}

$box = new PerfRegressionAliasBox();
$alias =& $box;

for ($i = 0; $i < 8; $i++) {
    $alias->bump();
    if ($i === 3) {
        $box->value = 10;
    }
    echo $alias->value, ':', $box->value, '|';
}
echo "\n";
