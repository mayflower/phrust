<?php
class Prompt28PrivateCloneWith {
    private $value;
}

$original = new Prompt28PrivateCloneWith();
$copy = clone($original, ["value" => 1]);
