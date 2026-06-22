<?php
// phase5-runtime: expect=pass
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

#[Phase5Component("service")]
final class Phase5Service
{
    use Phase5ServiceTrait;

    public function handle(Phase5Mode $mode, string $suffix): string
    {
        $label = $this->label();
        $modeName = $mode->name;
        $format = function () use ($label, $modeName, $suffix): string {
            return $label . ":" . $modeName . ":" . $suffix;
        };

        return $format();
    }
}

$reflection = new ReflectionClass(Phase5Service::class);
$attributes = $reflection->getAttributes();
echo $reflection->getName(), "\n";
echo $attributes[0]->getName(), ":", $attributes[0]->getArguments()[0], "\n";

$service = new Phase5Service();
echo $service->handle(Phase5Mode::Cli, "ok"), "\n";
