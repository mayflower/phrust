<?php
class PrivateCloneWithFixture {
    private $value;
}

$original = new PrivateCloneWithFixture();
$copy = clone($original, ["value" => 1]);
