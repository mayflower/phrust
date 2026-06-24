<?php
class ReadonlyCloneWithFixture {
    public readonly $value;
}

$original = new ReadonlyCloneWithFixture();
$copy = clone($original, ["value" => 1]);
