<?php
class PerfPolyA {
    public $value = 'A';
    public function value() {
        return $this->value;
    }
}

class PerfPolyB {
    public $value = 'B';
    public function value() {
        return $this->value;
    }
}

class PerfPolyC {
    public $value = 'C';
    public function value() {
        return $this->value;
    }
}

class PerfPolyD {
    public $value = 'D';
    public function value() {
        return $this->value;
    }
}

class PerfPolyE {
    public $value = 'E';
    public function value() {
        return $this->value;
    }
}

function perf_emit_poly($object) {
    echo $object->value(), ':', $object->value, '|';
}

$a = new PerfPolyA();
$b = new PerfPolyB();
$c = new PerfPolyC();
$d = new PerfPolyD();
$e = new PerfPolyE();

perf_emit_poly($a);
perf_emit_poly($b);
perf_emit_poly($a);
perf_emit_poly($b);
perf_emit_poly($c);
perf_emit_poly($d);
perf_emit_poly($e);
perf_emit_poly($a);
echo "\n";
