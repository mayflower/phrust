<?php
class Phase7PolyA {
    public $value = 'A';
    public function value() {
        return $this->value;
    }
}

class Phase7PolyB {
    public $value = 'B';
    public function value() {
        return $this->value;
    }
}

class Phase7PolyC {
    public $value = 'C';
    public function value() {
        return $this->value;
    }
}

class Phase7PolyD {
    public $value = 'D';
    public function value() {
        return $this->value;
    }
}

class Phase7PolyE {
    public $value = 'E';
    public function value() {
        return $this->value;
    }
}

function phase7_emit_poly($object) {
    echo $object->value(), ':', $object->value, '|';
}

$a = new Phase7PolyA();
$b = new Phase7PolyB();
$c = new Phase7PolyC();
$d = new Phase7PolyD();
$e = new Phase7PolyE();

phase7_emit_poly($a);
phase7_emit_poly($b);
phase7_emit_poly($a);
phase7_emit_poly($b);
phase7_emit_poly($c);
phase7_emit_poly($d);
phase7_emit_poly($e);
phase7_emit_poly($a);
echo "\n";
