<?php
// phase5-runtime: category=destructors expect=pass
class D {
    public string $name;

    public function __construct(string $name) {
        $this->name = $name;
    }

    public function __destruct() {
        echo "d:", $this->name, "\n";
    }
}

$a = new D("a");
$b = new D("b");
echo "body\n";
