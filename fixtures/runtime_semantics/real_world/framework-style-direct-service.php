<?php
// runtime-semantics: expect=pass
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

#[RuntimeSemanticsComponent("service")]
final class RuntimeSemanticsService
{
    use RuntimeSemanticsServiceTrait;

    public function handle(RuntimeSemanticsMode $mode, string $suffix): string
    {
        $label = $this->label();
        $modeName = $mode->name;
        $format = function () use ($label, $modeName, $suffix): string {
            return $label . ":" . $modeName . ":" . $suffix;
        };

        return $format();
    }
}

$reflection = new ReflectionClass(RuntimeSemanticsService::class);
$attributes = $reflection->getAttributes();
echo $reflection->getName(), "\n";
echo $attributes[0]->getName(), ":", $attributes[0]->getArguments()[0], "\n";

$service = new RuntimeSemanticsService();
echo $service->handle(RuntimeSemanticsMode::Cli, "ok"), "\n";
