<?php
// runtime-semantics: category=known_gaps expect=known_gap known_gap=E_PHP_RUNTIME_UNSUPPORTED_READONLY_ASYMMETRIC
// PHP reference: readonly properties can be initialized once and then read normally.
class ReadonlyFixtureBox
{
    public readonly int $value;

    public function __construct(int $value)
    {
        $this->value = $value;
    }
}

$box = new ReadonlyFixtureBox(9);
echo $box->value, "\n";
