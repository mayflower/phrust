<?php
class PerfFrameworkService {
    public function label($value) {
        return 'service:' . strtoupper($value);
    }
}

class PerfFrameworkContainer {
    private $core;

    public function __construct() {
        $this->core = new PerfFrameworkService();
    }

    public function getCore() {
        return $this->core;
    }
}

$container = new PerfFrameworkContainer();
for ($i = 0; $i < 4; $i++) {
    echo $container->getCore()->label('core'), "\n";
}
