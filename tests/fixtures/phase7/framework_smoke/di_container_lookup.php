<?php
class Phase7FrameworkService {
    public function label($value) {
        return 'service:' . strtoupper($value);
    }
}

class Phase7FrameworkContainer {
    private $core;

    public function __construct() {
        $this->core = new Phase7FrameworkService();
    }

    public function getCore() {
        return $this->core;
    }
}

$container = new Phase7FrameworkContainer();
for ($i = 0; $i < 4; $i++) {
    echo $container->getCore()->label('core'), "\n";
}
