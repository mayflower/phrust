<?php
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
