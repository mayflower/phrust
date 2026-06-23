<?php
#[Attribute(Attribute::TARGET_CLASS | Attribute::TARGET_METHOD)]
class Phase7FrameworkRoute {
    public function __construct(public string $path) {
    }
}

#[Phase7FrameworkRoute('/home')]
class Phase7FrameworkController {
    #[Phase7FrameworkRoute('/index')]
    public function index() {
    }
}

for ($i = 0; $i < 3; $i++) {
    $class = new ReflectionClass('Phase7FrameworkController');
    echo 'class=', $class->getAttributes()[0]->getName(), "\n";
    echo 'method=', $class->getMethods()[0]->getName(), "\n";
}
