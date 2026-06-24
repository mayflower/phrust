<?php
// runtime-semantics: expect=known_gap known_gap=E_PHP_RUNTIME_COMPOSER_AUTOLOAD_MATRIX
trait RuntimeSemanticsServiceTrait
{
    public function label(): string
    {
        return "service";
    }
}

enum RuntimeSemanticsMode
{
    case Cli;
}

spl_autoload_register(function (string $class): void {
    if ($class === "RuntimeSemanticsService") {
        include __DIR__ . "/_data/RuntimeSemanticsService.php";
    }
});

echo class_exists("RuntimeSemanticsService") ? "autoloaded\n" : "missing\n";

$reflection = new ReflectionClass(RuntimeSemanticsService::class);
$attributes = $reflection->getAttributes();
echo $reflection->getName(), "\n";
echo $attributes[0]->getName(), ":", $attributes[0]->getArguments()[0], "\n";

$service = new RuntimeSemanticsService();
echo $service->handle(RuntimeSemanticsMode::Cli, "ok"), "\n";
