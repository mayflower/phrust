<?php
// runtime-semantics: category=known_gaps expect=known_gap known_gap=E_PHP_RUNTIME_SERIALIZATION_STDLIB_GAP
// PHP reference: calls __serialize() and serializes the returned array.
class SerializeBoxFixture
{
    public function __serialize(): array
    {
        echo "serialize-hook|";
        return ["value" => 7];
    }
}
echo serialize(new SerializeBoxFixture()), "\n";
