<?php
class PerfDirectThrower {
    public int $calls = 0;

    public function fail(): int {
        $this->calls = $this->calls + 1;
        if ($this->calls <= 1) {
            return $this->calls;
        }
        throw new Exception("performance-direct-method");
    }
}

$object = new PerfDirectThrower();
for ($i = 0; $i < 2; $i++) {
    echo $object->fail(), "\n";
}
