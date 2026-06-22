<?php
// phase5-runtime: expect=known_gap known_gap=E_PHP_RUNTIME_COMPOSER_AUTOLOAD_MATRIX
trait Phase5ServiceTrait
{
    public function label(): string
    {
        return "service";
    }
}

enum Phase5Mode
{
    case Cli;
}

spl_autoload_register(function (string $class): void {
    if ($class === "Phase5Service") {
        include __DIR__ . "/_data/Phase5Service.php";
    }
});

echo class_exists("Phase5Service") ? "autoloaded\n" : "missing\n";

$reflection = new ReflectionClass(Phase5Service::class);
$attributes = $reflection->getAttributes();
echo $reflection->getName(), "\n";
echo $attributes[0]->getName(), ":", $attributes[0]->getArguments()[0], "\n";

$service = new Phase5Service();
echo $service->handle(Phase5Mode::Cli, "ok"), "\n";
