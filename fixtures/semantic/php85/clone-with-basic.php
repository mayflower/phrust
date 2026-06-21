<?php
class Prompt32CloneWithSubject
{
    public function __construct(public string $name = "old") {}
}

$copy = clone(new Prompt32CloneWithSubject(), ["name" => "new"]);
