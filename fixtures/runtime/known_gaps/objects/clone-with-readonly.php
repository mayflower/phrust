<?php
class Prompt28ReadonlyCloneWith {
    public readonly $value;
}

$original = new Prompt28ReadonlyCloneWith();
$copy = clone($original, ["value" => 1]);
