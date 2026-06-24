<?php
// runtime-semantics: expect=known_gap known_gap=E_PHP_RUNTIME_COMPOSER_STDLIB_MATRIX
#[RuntimeSemanticsRoute("/users/{id}")]
final class RuntimeSemanticsController
{
    public function __construct(public int $id)
    {
    }

    public function __invoke(RuntimeSemanticsMode $mode): string
    {
        return $mode->name . ":" . $this->id;
    }
}

enum RuntimeSemanticsMode
{
    case Http;
}

$class = new ReflectionClass(RuntimeSemanticsController::class);

// Real containers typically combine reflection metadata with stdlib helpers
// such as array_map(), count(), class_exists(), is_subclass_of(), and
// ReflectionClass::newInstanceArgs(). runtime-semantics keeps this as a handoff gap
// instead of pretending to run Composer packages.
$names = array_map(
    function ($attribute) {
        return $attribute->getName();
    },
    $class->getAttributes(),
);

echo $names[0], "\n";
