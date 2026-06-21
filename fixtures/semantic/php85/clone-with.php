<?php
class Prompt29CloneWithSubject
{
    public function __construct(public string $name = "old") {}
}

$copy = clone(new Prompt29CloneWithSubject(), ["name" => "new"]);
