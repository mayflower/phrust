<?php
// phase5-runtime: category=known_gaps expect=known_gap known_gap=E_PHP_RUNTIME_SERIALIZATION_PHASE6_GAP
// PHP reference: calls __serialize() and serializes the returned array.
class Prompt43SerializeBox
{
    public function __serialize(): array
    {
        echo "serialize-hook|";
        return ["value" => 7];
    }
}
echo serialize(new Prompt43SerializeBox()), "\n";
