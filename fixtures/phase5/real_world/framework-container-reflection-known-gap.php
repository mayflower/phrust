<?php
// phase5-runtime: expect=known_gap known_gap=E_PHP_RUNTIME_COMPOSER_STDLIB_MATRIX
#[Phase5Route("/users/{id}")]
final class Phase5Controller
{
    public function __construct(public int $id)
    {
    }

    public function __invoke(Phase5Mode $mode): string
    {
        return $mode->name . ":" . $this->id;
    }
}

enum Phase5Mode
{
    case Http;
}

$class = new ReflectionClass(Phase5Controller::class);

// Real containers typically combine reflection metadata with stdlib helpers
// such as array_map(), count(), class_exists(), is_subclass_of(), and
// ReflectionClass::newInstanceArgs(). Phase 5 keeps this as a handoff gap
// instead of pretending to run Composer packages.
$names = array_map(
    function ($attribute) {
        return $attribute->getName();
    },
    $class->getAttributes(),
);

echo $names[0], "\n";
