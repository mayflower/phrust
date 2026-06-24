<?php
class CloneWithSubjectFixture
{
    public function __construct(public string $name = "old") {}
}

$copy = clone(new CloneWithSubjectFixture(), ["name" => "new"]);
