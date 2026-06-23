<?php
class Phase7DirectThrower {
    public int $calls = 0;

    public function fail(): int {
        $this->calls = $this->calls + 1;
        if ($this->calls <= 1) {
            return $this->calls;
        }
        throw new Exception("phase7-direct-method");
    }
}

$object = new Phase7DirectThrower();
for ($i = 0; $i < 2; $i++) {
    echo $object->fail(), "\n";
}
