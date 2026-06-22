<?php
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
