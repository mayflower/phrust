<?php
#[Attribute(Attribute::TARGET_CLASS | Attribute::TARGET_METHOD)]
class PerfFrameworkRoute {
    public function __construct(public string $path) {
    }
}

#[PerfFrameworkRoute('/home')]
class PerfFrameworkController {
    #[PerfFrameworkRoute('/index')]
    public function index() {
    }
}

for ($i = 0; $i < 3; $i++) {
    $class = new ReflectionClass('PerfFrameworkController');
    echo 'class=', $class->getAttributes()[0]->getName(), "\n";
    echo 'method=', $class->getMethods()[0]->getName(), "\n";
}
